from pathlib import Path
from tempfile import TemporaryDirectory

import httpx

from ..converter import ConversionError, convert_pdf
from .api import download_file, get_file_path, send_document
from .models import ConversionJob

MAX_OUTPUT_BYTES = 50_000_000
TEMP_DIRECTORY_PREFIX = "slides-converter-"
DEFAULT_INPUT_NAME = "slides.pdf"
OUTPUT_NAME_SUFFIX = "-notes.pdf"


def process_job(job: ConversionJob, token: str, client: httpx.Client) -> None:
    with TemporaryDirectory(prefix=TEMP_DIRECTORY_PREFIX) as temporary_directory:
        directory = Path(temporary_directory)
        input_path = directory / DEFAULT_INPUT_NAME
        output_name = build_output_name(job.file_name)
        output_path = directory / output_name
        file_path = get_file_path(job.file_id, token, client)

        download_file(file_path, input_path, token, client)
        convert_pdf(input_path, output_path)
        validate_output_size(output_path)
        send_document(output_path, output_name, job, token, client)


def validate_output_size(output_path: Path) -> None:
    if output_path.stat().st_size > MAX_OUTPUT_BYTES:
        raise ConversionError("Processed PDF exceeds Telegram upload limit")


def build_output_name(file_name: str) -> str:
    safe_name = Path(file_name).name
    stem = Path(safe_name).stem.strip()

    if not stem or safe_name.casefold() == ".pdf":
        stem = Path(DEFAULT_INPUT_NAME).stem

    return f"{stem}{OUTPUT_NAME_SUFFIX}"
