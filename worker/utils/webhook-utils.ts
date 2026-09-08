import type { ConversionJob, Env } from "../index";

const MAX_INPUT_BYTES = 20_000_000;

export function hasValidWebhookSecret(request: Request, env: Env): boolean {
  return (
    request.headers.get("X-Telegram-Bot-Api-Secret-Token") ===
    env.TELEGRAM_WEBHOOK_SECRET
  );
}

export function isAllowedUser(userId: number, env: Env): boolean {
  return env.ALLOWED_TELEGRAM_USER_IDS.split(",").some(
    (value) => Number(value.trim()) === userId,
  );
}

export function isAcceptedPdf(job: ConversionJob): boolean {
  const hasPdfName = job.file_name.toLowerCase().endsWith(".pdf");
  const hasPdfMimeType = job.mime_type === "application/pdf";

  return (hasPdfName || hasPdfMimeType) && job.file_size <= MAX_INPUT_BYTES;
}

export function parseConversionJob(value: unknown): ConversionJob | null {
  const update = readRecord(value);
  const message = readRecord(update?.message);
  const document = readRecord(message?.document);
  const sender = readRecord(message?.from);
  const chat = readRecord(message?.chat);
  const messageId = readNumber(message?.message_id);
  const chatId = readNumber(chat?.id);
  const userId = readNumber(sender?.id);
  const fileId = readString(document?.file_id);
  const fileName = readString(document?.file_name) ?? "slides.pdf";
  const fileSize = readNumber(document?.file_size);
  const mimeType = readString(document?.mime_type) ?? "";

  if (
    messageId === null ||
    chatId === null ||
    userId === null ||
    fileId === null ||
    fileSize === null
  ) {
    return null;
  }

  return {
    chat_id: chatId,
    message_id: messageId,
    file_id: fileId,
    file_name: fileName,
    file_size: fileSize,
    mime_type: mimeType,
    user_id: userId,
  };
}

function readRecord(value: unknown): Record<string, unknown> | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function readNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isSafeInteger(value) ? value : null;
}

function readString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}
