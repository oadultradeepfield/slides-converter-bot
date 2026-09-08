from dataclasses import dataclass
from typing import Annotated, Literal

from pydantic import Field, TypeAdapter


class TelegramError(Exception):
    """Expected Telegram API failure."""


@dataclass(frozen=True, slots=True)
class ConversionJob:
    chat_id: int
    message_id: int
    file_id: Annotated[str, Field(min_length=1)]
    file_name: Annotated[str, Field(min_length=1)]
    file_size: Annotated[int, Field(ge=0)]


@dataclass(frozen=True, slots=True)
class TelegramFile:
    file_path: Annotated[str, Field(min_length=1)]


@dataclass(frozen=True, slots=True)
class TelegramFileResponse:
    ok: Literal[True]
    result: TelegramFile


@dataclass(frozen=True, slots=True)
class TelegramResponse:
    ok: bool
    description: str | None = None


TELEGRAM_FILE_RESPONSE_ADAPTER = TypeAdapter(TelegramFileResponse)
TELEGRAM_RESPONSE_ADAPTER = TypeAdapter(TelegramResponse)
