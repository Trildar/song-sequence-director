use cfg_if::cfg_if;

cfg_if! { if #[cfg(feature = "ssr")] {
    use std::convert::Infallible;

    use axum::{
        body::Body,
        response::IntoResponse,
        handler::HandlerWithoutStateExt,
        http::Request,
    };
    use tower_http::services::ServeDir;
    use leptos::*;
    use crate::error_template::ErrorTemplate;
    use crate::error_template::AppError;

    pub fn get_file_and_error_service(options: &LeptosOptions) -> impl tower::Service<Request<Body>, Response = impl IntoResponse, Error = Infallible, Future = impl Send> + Clone {
        let mut errors = Errors::default();
        errors.insert_with_default_key(AppError::NotFound);
        let error_handler = leptos_axum::render_app_to_stream(options.to_owned(), move |cx| view!{cx, <ErrorTemplate outside_errors=errors.clone()/>});

        ServeDir::new(&options.site_root).precompressed_br()
            .fallback(error_handler.into_service())
    }
}}
