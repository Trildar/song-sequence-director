use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::net::SocketAddr;
        use std::sync::Arc;

        use axum::extract::{ConnectInfo, Path, RawQuery, State, WebSocketUpgrade};
        use axum::response::{Response, IntoResponse};
        use axum::body::Body as AxumBody;
        use http::{HeaderMap, Request};
        use leptos::{provide_context, view};
        use leptos_axum::handle_server_fns_with_context;
        use song_sequence_director::app::{App, AppState, SectionTuple, section_socket};
        use tower_http::compression::CompressionLayer;

        #[tokio::main]
        async fn main() {
            use axum::{
                routing::{get, post},
                Router,
            };
            use leptos::*;
            use leptos_axum::{generate_route_list, LeptosRoutes};
            use song_sequence_director::app::*;
            use song_sequence_director::fileserv::get_file_and_error_service;
            use std::net::SocketAddr;

            simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

            // Setting get_configuration(None) means we'll be using cargo-leptos's env values
            // For deployment these variables are:
            // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
            // Alternately a file can be specified such as Some("Cargo.toml")
            // The file would need to be included with the executable when moved to deployment
            let conf = get_configuration(None).await.unwrap();
            let leptos_options = conf.leptos_options;
            let addr = leptos_options.site_addr;
            let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

            let (section_tx, section_rx) = tokio::sync::watch::channel((None, None));
            let app_state = AppState {
                leptos_options: leptos_options.clone(),
                section_tx: Arc::new(section_tx),
                section_rx
            };

            // build our application with a route
            let app = Router::new()
                .route("/ws", get(ws_handler))
                .route("/api/*fn_name", post(server_fn_handler))
                .leptos_routes_with_handler(routes, get(leptos_routes_handler))
                .with_state(app_state)
                .layer(CompressionLayer::new())
                .fallback_service(get_file_and_error_service(&leptos_options));

            // run our app with hyper
            // `axum::Server` is a re-export of `hyper::Server`
            log!("song director server v{} listening on http://{}", env!("CARGO_PKG_VERSION"), &addr);
            axum::Server::bind(&addr)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }

        async fn server_fn_handler(State(app_state): State<AppState>, path: Path<String>, headers: HeaderMap, raw_query: RawQuery, request: Request<AxumBody>) -> impl IntoResponse {
            handle_server_fns_with_context(path, headers, raw_query, move |cx| {
                provide_context(cx, app_state.section_tx.clone());
                provide_context(cx, app_state.section_rx.clone());
            }, request).await
        }

        async fn leptos_routes_handler(State(app_state): State<AppState>, req: Request<AxumBody>) -> Response {
            let handler = leptos_axum::render_app_to_stream_with_context(app_state.leptos_options.clone(), move |cx| {
                provide_context(cx, app_state.section_tx.clone());
                provide_context(cx, app_state.section_rx.clone());
            }, |cx| view! { cx, <App/> });

            handler(req).await.into_response()
        }

        async fn ws_handler(State(section_rx): State<tokio::sync::watch::Receiver<SectionTuple>>, ws: WebSocketUpgrade, ConnectInfo(socket_addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
            ws.on_upgrade(move |socket| section_socket(socket, section_rx, socket_addr))
        }
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
