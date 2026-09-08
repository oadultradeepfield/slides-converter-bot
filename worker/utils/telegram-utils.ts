import type { ConversionJob, Env } from "../index";

export async function sendMessage(
  job: ConversionJob,
  text: string,
  env: Env,
): Promise<void> {
  await fetch(
    `https://api.telegram.org/bot${env.TELEGRAM_BOT_TOKEN}/sendMessage`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        chat_id: job.chat_id,
        text,
        reply_parameters: { message_id: job.message_id },
      }),
    },
  ).catch(() => undefined);
}
