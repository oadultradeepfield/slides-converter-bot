use worker::*;

use crate::model::{is_allowed, parse_job};

#[event(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[event(fetch)]
pub async fn fetch(mut request: Request, env: Env, _context: Context) -> Result<Response> {
    if request.method() == Method::Get && request.path() == "/health" {
        return Response::from_json(
            &serde_json::json!({"status": "ok", "runtime": "rust-wasm", "version": env!("CARGO_PKG_VERSION")}),
        );
    }
    if request.method() != Method::Post || request.path() != "/telegram" {
        return Response::error("not found", 404);
    }
    let expected = env.secret("TELEGRAM_WEBHOOK_SECRET")?.to_string();
    if expected.is_empty()
        || request
            .headers()
            .get("X-Telegram-Bot-Api-Secret-Token")?
            .as_deref()
            != Some(&expected)
    {
        return Response::error("forbidden", 403);
    }
    let value = match request.json::<serde_json::Value>().await {
        Ok(value) => value,
        Err(_) => return Response::error("invalid JSON", 400),
    };
    let Some(job) = parse_job(value) else {
        return Response::ok("ok");
    };
    if !is_allowed(
        job.user_id,
        &env.secret("ALLOWED_TELEGRAM_USER_IDS")?.to_string(),
    ) {
        return Response::ok("ok");
    }
    let namespace = env.durable_object("CONVERSION_JOBS")?;
    let stub = namespace.get_by_name("converter")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(serde_json::to_string(&job)?.into()));
    let result = stub
        .fetch_with_request(Request::new_with_init("https://job/submit", &init)?)
        .await?;
    if result.status_code() != 200 {
        return Response::error("could not store job", 503);
    }
    Response::ok("ok")
}
