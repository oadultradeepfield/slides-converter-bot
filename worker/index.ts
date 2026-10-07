import { Container } from "@cloudflare/containers";

import { handleRequest } from "./utils/request-utils";

export interface Env {
  ALLOWED_TELEGRAM_USER_IDS: string;
  SLIDES_CONVERTER: DurableObjectNamespace<SlidesConverter>;
  TELEGRAM_BOT_TOKEN: string;
  TELEGRAM_WEBHOOK_SECRET: string;
}

export interface ConversionJob {
  chat_id: number;
  message_id: number;
  file_id: string;
  file_name: string;
  file_size: number;
  mime_type: string;
  user_id: number;
}

export class SlidesConverter extends Container<Env> {
  defaultPort = 8080;
  sleepAfter = "1m";
  pingEndpoint = "container/health";
  private readonly durableState: DurableObjectState;

  constructor(ctx: DurableObjectState<{}>, env: Env) {
    super(ctx, env);
    this.durableState = ctx;
    this.envVars = { TELEGRAM_BOT_TOKEN: env.TELEGRAM_BOT_TOKEN };
  }

  async fetch(request: Request): Promise<Response> {
    if (new URL(request.url).pathname !== "/jobs") {
      return super.fetch(request);
    }

    const job: ConversionJob = await request.clone().json();
    const jobKey = `job:${job.chat_id}:${job.message_id}`;
    const isNewJob = await this.durableState.storage.transaction(async (storage) => {
      if (await storage.get(jobKey)) {
        return false;
      }

      await storage.put(jobKey, Date.now());
      return true;
    });

    if (!isNewJob) {
      return new Response(null, { status: 204 });
    }

    return super.fetch(request);
  }
}

export default {
  fetch: handleRequest,
} satisfies ExportedHandler<Env>;
