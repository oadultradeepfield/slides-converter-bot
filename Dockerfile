FROM ghcr.io/astral-sh/uv:0.10.0 AS uv
FROM python:3.12-slim-bookworm

COPY --from=uv /uv /uvx /bin/

WORKDIR /app

COPY pyproject.toml uv.lock README.md ./
RUN uv sync --frozen --no-dev --no-install-project

COPY src ./src
RUN uv sync --frozen --no-dev

ENV PATH="/app/.venv/bin:$PATH"

EXPOSE 8080

CMD ["uvicorn", "slides_converter_bot.app:app", "--host", "0.0.0.0", "--port", "8080", "--workers", "1"]
