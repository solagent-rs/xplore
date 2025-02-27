use crate::error::Result;
use reqwest::multipart::Form;
use reqwest::{header::HeaderMap, Client, Method};
use serde::de::DeserializeOwned;

pub async fn request_api<T>(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    method: Method,
    body: Option<serde_json::Value>,
) -> Result<(T, HeaderMap)>
where
    T: DeserializeOwned,
{
    let mut request = client.request(method, url).headers(headers);

    if let Some(json_body) = body {
        request = request.json(&json_body);
    }

    let response = request.send().await?;

    if response.status().is_success() {
        let headers = response.headers().clone();
        let text = response.text().await?;
        let parsed: T = serde_json::from_str(&text)?;
        Ok((parsed, headers))
    } else {
        Err(crate::error::TwitterError::Api(format!("Request failed with status: {}", response.status())))
    }
}

pub async fn request_multipart_api<T>(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    form: Form,
) -> Result<(T, HeaderMap)>
where
    T: DeserializeOwned,
{
    let request = client.request(Method::POST, url).headers(headers).multipart(form);

    let response = request.send().await?;

    if response.status().is_success() {
        let headers = response.headers().clone();
        let text = response.text().await?;
        let parsed: T = serde_json::from_str(&text)?;
        Ok((parsed, headers))
    } else {
        Err(crate::error::TwitterError::Api(format!("Request failed with status: {}", response.status())))
    }
}

pub async fn request_form_api<T>(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    form_data: Vec<(String, String)>,
) -> Result<(T, HeaderMap)>
where
    T: DeserializeOwned,
{
    let request = client.request(Method::POST, url).headers(headers).form(&form_data);

    let response = request.send().await?;

    if response.status().is_success() {
        let headers = response.headers().clone();
        let text = response.text().await?;
        let parsed: T = serde_json::from_str(&text)?;
        Ok((parsed, headers))
    } else {
        Err(crate::error::TwitterError::Api(format!("Request failed with status: {}", response.status())))
    }
}
