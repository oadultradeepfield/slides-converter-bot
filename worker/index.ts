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

  constructor(ctx: DurableObjectState<{}>, env: Env) {
    super(ctx, env);
    this.envVars = { TELEGRAM_BOT_TOKEN: env.TELEGRAM_BOT_TOKEN };
  }
}

export default {
  fetch: handleRequest,
} satisfies ExportedHandler<Env>;
