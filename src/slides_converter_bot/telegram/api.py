from pathlib import Path

import httpx
from pydantic import ValidationError

from .models import (
    TELEGRAM_FILE_RESPONSE_ADAPTER,
    TELEGRAM_RESPONSE_ADAPTER,
    ConversionJob,
    TelegramError,
)

MAX_INPUT_BYTES = 20_000_000
TELEGRAM_API_URL = "https://api.telegram.org/bot{token}/{method}"
TELEGRAM_FILE_URL = "https://api.telegram.org/file/bot{token}/{file_path}"


def get_file_path(file_id: str, token: str, client: httpx.Client) -> str:
    response = client.get(
        TELEGRAM_API_URL.format(token=token, method="getFile"),
        params={"file_id": file_id},
    )
    response.raise_for_status()

    try:
        result = TELEGRAM_FILE_RESPONSE_ADAPTER.validate_json(response.text)
    except ValidationError as error:
        raise TelegramError("Telegram did not return a downloadable file") from error

    return result.result.file_path


def download_file(
    file_path: str,
    destination: Path,
    token: str,
    client: httpx.Client,
) -> None:
    url = TELEGRAM_FILE_URL.format(token=token, file_path=file_path)
    downloaded_bytes = 0

    with client.stream("GET", url) as response:
        response.raise_for_status()

        with destination.open("wb") as handle:
            for chunk in response.iter_bytes():
                downloaded_bytes += len(chunk)

                if downloaded_bytes > MAX_INPUT_BYTES:
                    raise TelegramError("Input PDF exceeds Telegram download limit")

                handle.write(chunk)


def send_document(
    output_path: Path,
    output_name: str,
    job: ConversionJob,
    token: str,
    client: httpx.Client,
) -> None:
    with output_path.open("rb") as handle:
        response = client.post(
            TELEGRAM_API_URL.format(token=token, method="sendDocument"),
            data={
                "chat_id": str(job.chat_id),
                "reply_parameters": f'{{"message_id":{job.message_id}}}',
            },
            files={"document": (output_name, handle, "application/pdf")},
        )

    validate_telegram_response(response)


def validate_telegram_response(response: httpx.Response) -> None:
    response.raise_for_status()

    try:
        result = TELEGRAM_RESPONSE_ADAPTER.validate_json(response.text)
    except ValidationError as error:
        raise TelegramError("Telegram returned an invalid response") from error

    if not result.ok:
        raise TelegramError(result.description or "Telegram rejected request")
