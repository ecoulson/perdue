use askama::Template;
use axum::response::{Html, IntoResponse};
use reqwest::StatusCode;

pub struct HtmlTemplate<T> {
    pub template: T,
}

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> axum::response::Response {
        match self.template.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error {}", err),
            )
                .into_response(),
        }
    }
}
