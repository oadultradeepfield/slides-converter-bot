use worker::*;

use crate::{
    model::{Job, accepts_pdf},
    pdf,
    queue::{Completion, JobQueue},
    telegram::Telegram,
};

#[durable_object]
pub struct ConversionJob {
    state: State,
    env: Env,
}

impl DurableObject for ConversionJob {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut request: Request) -> Result<Response> {
        if request.method() != Method::Post || request.path() != "/submit" {
            return Response::error("not found", 404);
        }
        let job = request.json::<Job>().await?;
        let queue = JobQueue::open(self.state.storage())?;
        queue.enqueue(&job)?;
        queue.ensure_alarm().await?;
        Response::ok("ok")
    }

    async fn alarm(&self) -> Result<Response> {
        let queue = JobQueue::open(self.state.storage())?;
        let Some(job) = queue.claim()? else {
            return Response::ok("ok");
        };
        let telegram = Telegram::new(self.env.secret("TELEGRAM_BOT_TOKEN")?.to_string());
        let start = Date::now().as_millis();
        let result = convert_and_reply(&job, &queue, &telegram).await;
        let completion = if result.is_ok() {
            Completion::Done
        } else {
            Completion::Failed
        };
        queue.complete(&job, completion)?;
        console_log!(
            "conversion success={} elapsed_ms={}",
            result.is_ok(),
            Date::now().as_millis() - start
        );
        if result.is_err() {
            // Network errors may contain bot-token URLs. Log only the outcome.
            let _ = telegram
                .send_message(
                    &job,
                    "Conversion failed. Please send the PDF again and retry.",
                )
                .await;
        }
        queue.ensure_alarm().await?;
        Response::ok("ok")
    }
}

async fn convert_and_reply(job: &Job, queue: &JobQueue, telegram: &Telegram) -> Result<()> {
    if !accepts_pdf(job) {
        queue.mark_sending(job)?;
        return telegram
            .send_message(job, "Please send a PDF file no larger than 20 MB.")
            .await;
    }
    let input = telegram.download_pdf(&job.file_id).await?;
    let output =
        pdf::convert_pdf(input).map_err(|_| Error::RustError("PDF conversion failed".into()))?;
    queue.mark_sending(job)?;
    telegram.send_document(job, output).await
}
