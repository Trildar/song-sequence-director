use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use axum::extract::{ConnectInfo, WebSocketUpgrade};
        use axum::response::IntoResponse;
        use song_sequence_director::app::section_socket;

        use std::net::SocketAddr;

        #[tokio::main]
        async fn main() {
            use axum::{
                extract::Extension,
                routing::{get, post},
                Router,
            };
            use leptos::*;
            use leptos_axum::{generate_route_list, LeptosRoutes};
            use song_sequence_director::app::*;
            use song_sequence_director::fileserv::file_and_error_handler;
            use std::net::SocketAddr;
            use std::sync::Arc;

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

            // Register server functions
            let _ = GetSection::register();
            let _ = SetSection::register();

            // build our application with a route
            let app = Router::new()
                .route("/ws", get(ws_handler))
                .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
                .leptos_routes(leptos_options.clone(), routes, |cx| view! { cx, <App/> })
                .fallback(file_and_error_handler)
                .layer(Extension(Arc::new(leptos_options)));

            // run our app with hyper
            // `axum::Server` is a re-export of `hyper::Server`
            log!("song director server v{} listening on http://{}", env!("CARGO_PKG_VERSION"), &addr);
            axum::Server::bind(&addr)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }

        async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(socket_addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
            ws.on_upgrade(move |socket| section_socket(socket, socket_addr))
        }
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
