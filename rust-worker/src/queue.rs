use serde::Deserialize;
use worker::{Result, Storage};

use crate::model::Job;

pub(crate) struct JobQueue {
    storage: Storage,
}

pub(crate) enum Completion {
    Done,
    Failed,
}

#[derive(Deserialize)]
struct StoredJob {
    body: String,
}

#[derive(Deserialize)]
struct PendingCount {
    count: u32,
}

const ALARM_DELAY_MS: i64 = 100;

impl JobQueue {
    pub fn open(storage: Storage) -> Result<Self> {
        let sql = storage.sql();
        sql.exec("CREATE TABLE IF NOT EXISTS job (id TEXT PRIMARY KEY, body TEXT NOT NULL, status TEXT NOT NULL, created INTEGER NOT NULL DEFAULT (unixepoch()))", None)?;
        sql.exec(
            "CREATE INDEX IF NOT EXISTS job_status ON job(status, created)",
            None,
        )?;
        Ok(Self { storage })
    }

    pub fn enqueue(&self, job: &Job) -> Result<()> {
        let sql = self.storage.sql();
        // Telegram retries updates for at most 24 hours; retain tombstones for seven days.
        sql.exec("DELETE FROM job WHERE status IN ('done', 'failed') AND created < unixepoch()-7*24*60*60", None)?;
        sql.exec(
            "INSERT OR IGNORE INTO job(id, body, status) VALUES (?, ?, 'queued')",
            Some(vec![
                job_key(job).into(),
                serde_json::to_string(job)?.into(),
            ]),
        )?;
        Ok(())
    }

    pub fn claim(&self) -> Result<Option<Job>> {
        let sql = self.storage.sql();
        // A crash after sending starts has an uncertain outcome. Never send that job again.
        sql.exec(
            "UPDATE job SET status='failed', body='' WHERE status='sending'",
            None,
        )?;
        let rows: Vec<StoredJob> = sql.exec("UPDATE job SET status='processing' WHERE id=(SELECT id FROM job WHERE status IN ('queued', 'processing') ORDER BY created, rowid LIMIT 1) RETURNING body", None)?.to_array()?;
        rows.first()
            .map(|row| serde_json::from_str(&row.body).map_err(Into::into))
            .transpose()
    }

    pub fn mark_sending(&self, job: &Job) -> Result<()> {
        self.storage.sql().exec(
            "UPDATE job SET status='sending' WHERE id=?",
            Some(vec![job_key(job).into()]),
        )?;
        Ok(())
    }

    pub fn complete(&self, job: &Job, completion: Completion) -> Result<()> {
        let status = match completion {
            Completion::Done => "done",
            Completion::Failed => "failed",
        };
        self.storage.sql().exec(
            "UPDATE job SET status=?, body='' WHERE id=?",
            Some(vec![status.into(), job_key(job).into()]),
        )?;
        Ok(())
    }

    pub async fn ensure_alarm(&self) -> Result<()> {
        let pending: PendingCount = self
            .storage
            .sql()
            .exec(
                "SELECT COUNT(*) AS count FROM job WHERE status IN ('queued', 'processing')",
                None,
            )?
            .one()?;
        if pending.count > 0 && self.storage.get_alarm().await?.is_none() {
            self.storage.set_alarm(ALARM_DELAY_MS).await?;
        }
        Ok(())
    }
}

fn job_key(job: &Job) -> String {
    format!("{}:{}", job.chat_id, job.message_id)
}
