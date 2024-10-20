use serde::de::DeserializeOwned;
use tiny_http::{Header, Request};

pub fn find_header<'a>(request: &'a Request, name: &str) -> Option<&'a Header> {
    request
        .headers()
        .iter()
        .find(|header| header.field.to_string().to_lowercase() == name.to_lowercase())
}

pub fn extract_query<T>(url: &str) -> Result<T, serde_qs::Error>
where
    T: DeserializeOwned,
{
    serde_qs::from_str::<T>(&url.to_string().split("?").skip(1).next().unwrap_or(""))
}

pub fn parse_form_data<T>(request: &mut Request) -> Result<T, serde_urlencoded::de::Error>
where
    T: DeserializeOwned,
{
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body).unwrap();

    serde_urlencoded::from_str(&body)
}
