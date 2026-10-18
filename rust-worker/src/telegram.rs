use futures_util::StreamExt;
use serde::{Deserialize, de::DeserializeOwned};
use worker::*;

use crate::{
    model::{Job, output_name},
    pdf::MAX_INPUT_BYTES,
};

pub(crate) struct Telegram {
    token: String,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    ok: bool,
    result: Option<T>,
}

#[derive(Deserialize)]
struct TelegramFile {
    file_path: String,
}

const API_ORIGIN: &str = "https://api.telegram.org";

impl Telegram {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub async fn download_pdf(&self, file_id: &str) -> Result<Vec<u8>> {
        let response = self
            .post_json("getFile", &serde_json::json!({"file_id": file_id}))
            .await?;
        let path = read_result::<TelegramFile>(response).await?.file_path;
        if path.split('/').any(|part| part == "..") {
            return Err(Error::RustError("Invalid file path".into()));
        }
        let mut response = Fetch::Url(Url::parse(&format!(
            "{API_ORIGIN}/file/bot{}/{path}",
            self.token
        ))?)
        .send()
        .await?;
        check_status(&response)?;
        read_limited_body(&mut response).await
    }

    pub async fn send_message(&self, job: &Job, text: &str) -> Result<()> {
        let response = self.post_json("sendMessage", &serde_json::json!({
            "chat_id": job.chat_id, "text": text, "reply_parameters": {"message_id": job.message_id},
        })).await?;
        read_result::<serde_json::Value>(response).await?;
        Ok(())
    }

    pub async fn send_document(&self, job: &Job, output: Vec<u8>) -> Result<()> {
        let form = create_document_form(job, output)?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post).with_body(Some(form.into()));
        let response = Fetch::Request(Request::new_with_init(
            &self.api_url("sendDocument"),
            &init,
        )?)
        .send()
        .await?;
        read_result::<serde_json::Value>(response).await?;
        Ok(())
    }

    async fn post_json(&self, method: &str, body: &serde_json::Value) -> Result<Response> {
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(body.to_string().into()));
        Fetch::Request(Request::new_with_init(&self.api_url(method), &init)?)
            .send()
            .await
    }

    fn api_url(&self, method: &str) -> String {
        format!("{API_ORIGIN}/bot{}/{method}", self.token)
    }
}

async fn read_limited_body(response: &mut Response) -> Result<Vec<u8>> {
    let mut stream = response.stream()?;
    let mut input = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if chunk.len() > MAX_INPUT_BYTES.saturating_sub(input.len()) {
            return Err(Error::RustError("Input PDF exceeds 20 MB".into()));
        }
        input.extend_from_slice(&chunk);
    }
    Ok(input)
}

async fn read_result<T: DeserializeOwned>(mut response: Response) -> Result<T> {
    check_status(&response)?;
    let body: ApiResponse<T> = response.json().await?;
    body.result
        .filter(|_| body.ok)
        .ok_or_else(|| Error::RustError("Telegram rejected request".into()))
}

fn check_status(response: &Response) -> Result<()> {
    if (200..300).contains(&response.status_code()) {
        return Ok(());
    }
    Err(Error::RustError("Telegram HTTP request failed".into()))
}

fn create_document_form(job: &Job, output: Vec<u8>) -> Result<web_sys::FormData> {
    let form = web_sys::FormData::new()?;
    form.append_with_str("chat_id", &job.chat_id.to_string())?;
    form.append_with_str(
        "reply_parameters",
        &serde_json::json!({"message_id": job.message_id}).to_string(),
    )?;
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/pdf");
    let bytes = js_sys::Uint8Array::from(output.as_slice());
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
        &js_sys::Array::of1(&bytes),
        &options,
    )?;
    form.append_with_blob_and_filename("document", &blob, &output_name(&job.file_name))?;
    Ok(form)
}
