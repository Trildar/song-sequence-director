use std::num::NonZeroUsize;

use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type SectionTuple = (Option<char>, Option<NonZeroUsize>);

#[derive(Clone, Debug, Error, Serialize, Deserialize)]
enum SectionLoadError {
    #[error("could not read location host")]
    LocationHostError,
    #[error("could not open WebSocket: {0}")]
    WebSocketOpenError(String),
    #[error("error receiving message from WebSocket: {0}")]
    WebSocketError(String),
    #[error("server error: {0}")]
    ServerError(#[from] ServerFnErrorErr),
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::net::SocketAddr;
        use std::sync::Arc;

        use axum::extract::ws::{self, WebSocket};
        use axum::extract::FromRef;
        use futures::StreamExt;

        #[derive(FromRef, Clone)]
        pub struct AppState {
            pub leptos_options: LeptosOptions,
            pub section_tx: Arc<tokio::sync::watch::Sender<SectionTuple>>,
            pub section_rx: tokio::sync::watch::Receiver<SectionTuple>,
        }

        fn get_section_tx(cx: Scope) -> Result<Arc<tokio::sync::watch::Sender<SectionTuple>>, ServerFnError> {
            use_context::<Arc<tokio::sync::watch::Sender<SectionTuple>>>(cx).ok_or_else(|| ServerFnError::ServerError("Section TX missing".to_string()))
        }

        fn get_section_rx(cx: Scope) -> Result<tokio::sync::watch::Receiver<SectionTuple>, ServerFnError> {
            use_context::<tokio::sync::watch::Receiver<SectionTuple>>(cx).ok_or_else(|| ServerFnError::ServerError("Section RX missing".to_string()))
        }

        pub async fn section_socket(mut socket: WebSocket, mut section_rx: tokio::sync::watch::Receiver<SectionTuple>, socket_addr: SocketAddr) {
            loop {
                tokio::select! {
                    changed = section_rx.changed() => if changed.is_ok() {
                        let message = section_segments_to_string(&section_rx.borrow());
                        log::debug!("Sending {}", message);
                        if let Err(err) = socket.send(ws::Message::Text(message)).await {
                            log::warn!("Error sending to {}: {}", socket_addr, err);
                            return;
                        }
                    } else {
                        // Channel has closed. Should never actually happen
                        let _ = socket.close().await;
                        return;
                    },
                    Some(Ok(ws::Message::Close(_))) = socket.next() => {
                        log::debug!("Socket with {} closed", socket_addr);
                        return;
                    }
                }
            }
        }
    }
}

fn section_segments_to_string(segments: &SectionTuple) -> String {
    if let Some(sec) = segments.0 {
        if let Some(num) = segments.1 {
            format!("{}{}", sec, num)
        } else {
            sec.to_string()
        }
    } else {
        "".to_string()
    }
}

#[server(GetSection, "/api", "Cbor")]
async fn get_section(cx: Scope) -> Result<SectionTuple, ServerFnError> {
    Ok(get_section_rx(cx)?.borrow().clone())
}

#[server(SetSection, "/api", "Cbor")]
async fn set_section(cx: Scope, section: SectionTuple) -> Result<(), ServerFnError> {
    log::debug!("Update section to {:?}", section);
    let tx = get_section_tx(cx)?;
    tx.send_modify(|s| *s = section);

    Ok(())
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);

    view! {
        cx,

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/song-sequence-director.css"/>

        // sets the document title
        <Title text="Song Director"/>

        <Link rel="icon" type_="image/png" href="/favicon.png"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <Director/> }/>
                    <Route path="view" view=|cx| view! {cx, <SectionDisplay/>}/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn Director(cx: Scope) -> impl IntoView {
    let section_resource = create_resource(cx, || (), move |_| get_section(cx));
    let set_section_action = create_server_action::<SetSection>(cx);

    let change_section_type = move |ch| {
        let new_section = (Some(ch), None);
        section_resource.set(Ok(new_section));
        set_section_action.dispatch(SetSection {
            section: new_section,
        });
    };
    let clear_section = move || {
        let new_section = (None, None);
        section_resource.set(Ok(new_section));
        set_section_action.dispatch(SetSection {
            section: new_section,
        });
    };
    let set_section_number = move |num| {
        section_resource.update(|sec| {
            if let Some(Ok(section)) = sec {
                section.1 = num;
            }
        });
        let new_section = section_resource.read(cx).unwrap().unwrap();
        set_section_action.dispatch(SetSection {
            section: new_section,
        });
    };
    let section_display = move || {
        let Some(section_segments) = section_resource.read(cx) else {
            return Ok::<_, ServerFnError>("\u{200b}".to_string());
        };
        let section_string = section_segments_to_string(&section_segments?);
        if section_string.is_empty() {
            // Zero-width space so that the vertical space is reserved when not displaying anything
            Ok("\u{200b}".to_string())
        } else {
            Ok(section_string)
        }
    };

    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            use leptos_dom::helpers::location;
            use leptos::spawn_local;
            use futures::StreamExt;

            let socket_stream_result = location().host().map_err(|_| SectionLoadError::LocationHostError)
                .and_then(|host| {
                    gloo_net::websocket::futures::WebSocket::open(&format!("ws://{}/ws", host))
                        .map_err(|err| SectionLoadError::WebSocketOpenError(err.to_string()))
                    });
            match socket_stream_result {
                Ok(mut socket_stream) =>
                    spawn_local(async move {
                        loop {
                            match socket_stream.next().await {
                                Some(Ok(gloo_net::websocket::Message::Text(message))) => {
                                    let mut message_chars = message.chars();
                                    let section_type = message_chars.next();
                                    let section_number = message_chars.as_str().parse::<NonZeroUsize>().ok();
                                    section_resource.set(Ok((section_type, section_number)));
                                },
                                Some(Err(err)) => {
                                    warn!("Error receiving message from WebSocket: {}", err);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }),
                Err(err) => warn!("{}", err),
            }
        }
    }

    view! { cx,
        <div class="director-container">
            <Suspense
                fallback= move || view! { cx, <div class="section-display">"\u{200b}"</div>}
            >
                <div class="section-display">{section_display}</div>
            </Suspense>
            <div class="director-buttons">
                <button on:click=move |_| change_section_type('C')>"C"</button>
                <button on:click=move |_| change_section_type('V')>"V"</button>
                <button on:click=move |_| change_section_type('B')>"B"</button>
                <button on:click=move |_| change_section_type('P')>"P"</button>
                <button on:click=move |_| change_section_type('W')>"W"</button>
                <button on:click=move |_| change_section_type('E')>"E"</button>
                <button on:click=move |_| change_section_type('X')>"X"</button>
                <button on:click=move |_| set_section_number(NonZeroUsize::new(1))>"1"</button>
                <button on:click=move |_| set_section_number(NonZeroUsize::new(2))>"2"</button>
                <button on:click=move |_| set_section_number(NonZeroUsize::new(3))>"3"</button>
                <button on:click=move |_| set_section_number(NonZeroUsize::new(4))>"4"</button>
                <button on:click=move |_| set_section_number(NonZeroUsize::new(5))>"5"</button>
                <button on:click=move |_| change_section_type('R')>"R"</button>
                <button on:click=move |_| clear_section()>"-"</button>
            </div>
        </div>
    }
}

#[component]
fn SectionDisplay(cx: Scope) -> impl IntoView {
    let section_resource = create_resource(
        cx,
        || (),
        move |_| async move {
            Ok::<_, SectionLoadError>(section_segments_to_string(
                &get_section(cx)
                    .await
                    .map_err(|err| SectionLoadError::from(ServerFnErrorErr::from(err)))?,
            ))
        },
    );
    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            use leptos_dom::helpers::location;
            use leptos::spawn_local;
            use futures::StreamExt;

            let socket_stream_result = location().host().map_err(|_| SectionLoadError::LocationHostError)
                .and_then(|host| {
                    gloo_net::websocket::futures::WebSocket::open(&format!("ws://{}/ws", host))
                        .map_err(|err| SectionLoadError::WebSocketOpenError(err.to_string()))
                    });
            match socket_stream_result {
                Ok(mut socket_stream) =>
                    spawn_local(async move {
                        loop {
                            match socket_stream.next().await {
                                Some(Ok(gloo_net::websocket::Message::Text(message))) => section_resource.set(Ok(message)),
                                Some(Err(err)) => {
                                    section_resource.set(Err(SectionLoadError::WebSocketError(err.to_string())));
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }),
                Err(err) => section_resource.set(Err(err)),
            }
        }
    }

    view! { cx,
        <Title text="Song Director - View" />
        <Suspense
            fallback= move || view! { cx, <div class="section-display">"\u{200b}"</div>}
        >
            <ErrorBoundary
                fallback= move |cx, errors| {
                    let errors: Vec<leptos::error::Error> = errors()
                        .into_iter()
                        .map(|(_, v)| v)
                        .collect();
                    view! { cx,
                        <h1>{if errors.len() > 1 {"Errors"} else {"Error"}}</h1>
                        <For
                            // a function that returns the items we're iterating over; a signal is fine
                            each= move || {errors.clone().into_iter().enumerate()}
                            // a unique key for each item as a reference
                            key=|(index, _error)| *index
                            // renders each item to a view
                            view= move |cx, error| {
                                let error_string = error.1.to_string();
                                view! {
                                    cx,
                                    <p>{error_string}</p>
                                }
                            }
                        />
                    }
                }
            >
                <div class="section-display">{move || section_resource.read(cx)}</div>
            </ErrorBoundary>
        </Suspense>
    }
}
