use std::{num::NonZeroUsize, sync::Arc};

use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use thiserror::Error;

use crate::error_template::ErrorTemplate;

type SectionTuple = (Option<char>, Option<NonZeroUsize>);

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug, Error)]
enum SectionSocketError {
    #[error("could not read location host")]
    LocationHostError,
    #[error("could not open WebSocket ({0})")]
    WebSocketOpenError(Arc<str>),
    #[error("error receiving message from WebSocket ({0})")]
    WebSocketError(Arc<str>),
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::net::SocketAddr;
        use std::sync::OnceLock;

        use axum::extract::ws::{self, WebSocket};
        use futures::StreamExt;

        static SECTION_CHANNEL: OnceLock<tokio::sync::watch::Sender<SectionTuple>> = OnceLock::new();

        fn get_section_channel() -> &'static tokio::sync::watch::Sender<SectionTuple> {
            SECTION_CHANNEL.get_or_init(|| tokio::sync::watch::channel((None, None)).0)
        }

        pub async fn section_socket(mut socket: WebSocket, socket_addr: SocketAddr) {
            let mut section_rx = get_section_channel().subscribe();

            let message = section_segments_to_string(&section_rx.borrow());
            log::debug!("Sending {}", message);
            if let Err(err) = socket.send(ws::Message::Text(message)).await {
                log::warn!("Error sending to {}: {}", socket_addr, err);
                return;
            }
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
                        // SECTION_CHANNEL has closed. Should never actually happen
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

#[server(SetSection, "/api", "Cbor")]
async fn set_section(section: SectionTuple) -> Result<(), ServerFnError> {
    log::debug!("Update section to {:?}", section);
    let tx = get_section_channel();
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
    let (section_type, set_section_type) = create_signal(cx, None::<char>);
    let (section_number, set_section_number) = create_signal(cx, None::<NonZeroUsize>);
    let change_section_type = move |ch| {
        cx.batch(|| {
            set_section_type(Some(ch));
            set_section_number(None);
        });
    };
    let section_display = move || section_segments_to_string(&(section_type(), section_number()));
    let _update_section =
        create_resource(cx, move || (section_type(), section_number()), set_section);

    view! { cx,
        <h1>"Welcome to Leptos!"</h1>
        <h2>{section_display}</h2>
        <button on:click=move |_| change_section_type('C')>"C"</button>
        <button on:click=move |_| change_section_type('V')>"V"</button>
        <button on:click=move |_| set_section_number(NonZeroUsize::new(1))>"1"</button>
        <button on:click=move |_| set_section_number(NonZeroUsize::new(2))>"2"</button>
    }
}

#[component]
fn SectionDisplay(cx: Scope) -> impl IntoView {
    cfg_if! {
        if #[cfg(feature = "ssr")] {
            let (section_string, _) = create_signal(cx, None::<String>);
        } else {
            use leptos_dom::helpers::location;
            use futures::{future::ready, StreamExt};

            let socket_stream = location().host().map_err(|_| SectionSocketError::LocationHostError)
                .and_then(|host| {
                    gloo_net::websocket::futures::WebSocket::open(&format!("ws://{}/ws", host))
                        .map_err(|err| SectionSocketError::WebSocketOpenError(Arc::from(err.to_string().as_str())))
                    });
            let section_stream_signal = socket_stream.map(|socket| {
                let s = socket.filter_map(|message_result| ready(match message_result {
                    Ok(gloo_net::websocket::Message::Text(message)) => Some(Ok(message)),
                    Err(err) => Some(Err(SectionSocketError::WebSocketError(Arc::from(err.to_string().as_str())))),
                    _ => None
                }));
                create_signal_from_stream(cx, s)
            });
            let section_string = section_stream_signal.unwrap_or_else(|err| {
                let (s, _) = create_signal(cx, Some(Err(err)));
                s
            });
        }
    }

    view! { cx,
        <ErrorBoundary
            fallback=move |_, errors| view! {cx, <ErrorTemplate errors=errors/>}
        >
        <span>{section_string}</span>
        </ErrorBoundary>
    }
}
