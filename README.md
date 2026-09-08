# slides-converter-bot

Personal Telegram bot that turns slide PDFs into A4 notes with three slides and dotted writing areas per page.

## Limits

- Input: PDF document up to 20 MB, matching Telegram Bot API download limit.
- Output: processed PDF up to 50 MB, matching Telegram `sendDocument` limit.
- One Cloudflare `basic` container processes one request at a time.
- No application retries or queue. Failed conversions ask user to send PDF again.

## Requirements

- Python 3.12+
- [uv](https://docs.astral.sh/uv/)
- Node.js 20+
- Docker
- Cloudflare Workers Paid plan with Containers
- Telegram bot token from BotFather

## Local setup

```bash
make install
cp .dev.vars.example .dev.vars
make check
npm run dev
```

Set these values in `.dev.vars`:

```env
TELEGRAM_BOT_TOKEN=123456:replace-me
TELEGRAM_WEBHOOK_SECRET=replace-with-random-secret
ALLOWED_TELEGRAM_USER_IDS=123456789,987654321
```

Only listed Telegram user IDs may submit conversions.

## Deploy

Authenticate, add secrets, then deploy:

```bash
npx wrangler login
npx wrangler secret put TELEGRAM_BOT_TOKEN
npx wrangler secret put TELEGRAM_WEBHOOK_SECRET
npx wrangler secret put ALLOWED_TELEGRAM_USER_IDS
npm run deploy
```

Register webhook after deploy:

```bash
curl --request POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/setWebhook" \
  --data-urlencode "url=https://slides-converter-bot.<workers-subdomain>.workers.dev/telegram" \
  --data-urlencode "secret_token=${TELEGRAM_WEBHOOK_SECRET}" \
  --data-urlencode 'allowed_updates=["message"]'
```

Send PDF to bot. Bot replies with `<original-name>-notes.pdf` or failure message asking for manual retry.

## Development

| Command | Purpose |
| --- | --- |
| `make fmt` | Format and auto-fix Python |
| `make lint` | Check Python formatting and lint |
| `make types` | Run strict mypy |
| `make test` | Run pytest |
| `make worker-check` | Type-check Cloudflare Worker |
| `make check` | Run every check |
| `make install-hooks` | Install project pre-commit hook |

Python core uses temporary container disk only. Worker handles webhook authentication, user allowlist, input filtering, and container routing. FastAPI container downloads, converts, and returns result through Telegram.

