import os

import httpx
from fastapi import FastAPI, Response

from .telegram import ConversionJob, process_job

BOT_TOKEN_ENVIRONMENT_VARIABLE = "TELEGRAM_BOT_TOKEN"
REQUEST_TIMEOUT_SECONDS = 300.0
CONNECT_TIMEOUT_SECONDS = 10.0
NO_CONTENT_STATUS = 204

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.get("/health")
def check_health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/jobs", status_code=NO_CONTENT_STATUS)
def process_conversion(job: ConversionJob) -> Response:
    token = load_bot_token()
    timeout = httpx.Timeout(
        REQUEST_TIMEOUT_SECONDS,
        connect=CONNECT_TIMEOUT_SECONDS,
    )

    with httpx.Client(timeout=timeout) as client:
        process_job(job, token, client)

    return Response(status_code=NO_CONTENT_STATUS)


def load_bot_token() -> str:
    token = os.environ.get(BOT_TOKEN_ENVIRONMENT_VARIABLE)

    if not token:
        raise RuntimeError(f"{BOT_TOKEN_ENVIRONMENT_VARIABLE} is required")

    return token
