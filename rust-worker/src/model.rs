use serde::{Deserialize, Serialize};

use crate::pdf::MAX_INPUT_BYTES;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Job {
    pub chat_id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub file_id: String,
    pub file_name: String,
    pub file_size: usize,
    pub mime_type: String,
}

#[derive(Deserialize)]
struct Update {
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    message_id: i64,
    chat: Identity,
    from: Identity,
    document: Document,
}

#[derive(Deserialize)]
struct Identity {
    id: i64,
}

#[derive(Deserialize)]
struct Document {
    file_id: String,
    file_size: usize,
    file_name: Option<String>,
    mime_type: Option<String>,
}

pub fn parse_job(value: serde_json::Value) -> Option<Job> {
    let update: Update = serde_json::from_value(value).ok()?;
    let message = update.message?;
    if message.document.file_id.is_empty() || message.message_id <= 0 || message.from.id <= 0 {
        return None;
    }
    Some(Job {
        chat_id: message.chat.id,
        message_id: message.message_id,
        user_id: message.from.id,
        file_id: message.document.file_id,
        file_name: message
            .document
            .file_name
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "slides.pdf".into()),
        file_size: message.document.file_size,
        mime_type: message.document.mime_type.unwrap_or_default(),
    })
}

pub fn is_allowed(user_id: i64, allowlist: &str) -> bool {
    allowlist
        .split(',')
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .any(|id| id == user_id)
}

pub fn accepts_pdf(job: &Job) -> bool {
    job.file_size <= MAX_INPUT_BYTES
        && (job.file_name.to_lowercase().ends_with(".pdf") || job.mime_type == "application/pdf")
}

pub fn output_name(file_name: &str) -> String {
    let base = file_name.rsplit('/').next().unwrap_or("slides.pdf");
    let stem = base.rsplit_once('.').map_or(base, |(stem, _)| stem).trim();
    let stem = if stem.is_empty() { "slides" } else { stem };
    format!("{stem}-notes.pdf")
}
