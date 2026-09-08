import { getContainer } from "@cloudflare/containers";

import type { ConversionJob, Env } from "../index";
import {
  hasValidWebhookSecret,
  isAcceptedPdf,
  isAllowedUser,
  parseConversionJob,
} from "./webhook-utils";
import { sendMessage } from "./telegram-utils";

export async function handleRequest(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url);

  if (request.method === "GET" && url.pathname === "/health") {
    return respondOk();
  }

  if (request.method !== "POST" || url.pathname !== "/telegram") {
    return new Response("not found", { status: 404 });
  }

  if (!hasValidWebhookSecret(request, env)) {
    return new Response("forbidden", { status: 403 });
  }

  const body: unknown = await request.json();
  const job = parseConversionJob(body);

  if (job === null || !isAllowedUser(job.user_id, env)) {
    return respondOk();
  }

  if (!isAcceptedPdf(job)) {
    await sendMessage(job, "Please send a PDF file no larger than 20 MB.", env);
    return respondOk();
  }

  await processConversion(job, env);
  return respondOk();
}

async function processConversion(job: ConversionJob, env: Env): Promise<void> {
  try {
    const container = getContainer(env.SLIDES_CONVERTER);
    const response = await container.fetch("http://container/jobs", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        chat_id: job.chat_id,
        message_id: job.message_id,
        file_id: job.file_id,
        file_name: job.file_name,
        file_size: job.file_size,
      }),
    });

    if (!response.ok) {
      await notifyFailure(job, env);
    }
  } catch {
    await notifyFailure(job, env);
  }
}

async function notifyFailure(job: ConversionJob, env: Env): Promise<void> {
  await sendMessage(
    job,
    "Conversion failed. Please send the PDF again and retry.",
    env,
  );
}

function respondOk(): Response {
  return new Response("ok");
}
