// Temporary migration tests: run the compiled Wasm inside workerd, never against Telegram.
import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, rm, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import { test } from "node:test";
import { Miniflare, Log, LogLevel, convertV4MiniflareOptions } from "miniflare";

const root = resolve(import.meta.dirname, "..");
const temporary = await mkdtemp(join(tmpdir(), "slides-worker-migration-"));
const fixture = join(temporary, "source.pdf");
const create = spawnSync("uv", ["run", "python", "-c",
  "from pathlib import Path; from migration.test_parity import create_deck; import sys; create_deck(Path(sys.argv[1]), 4)", fixture,
], { cwd: root, encoding: "utf8" });
assert.equal(create.status, 0, create.stderr);
const input = await readFile(fixture);
const documents = [];
const messages = [];
const downloads = [];
let releaseDownload;
let blockDownload = null;
let uploadFails = false;
let blockUpload = null;
let releaseUpload;

function createRuntime() {
  return new Miniflare(convertV4MiniflareOptions({
    log: new Log(LogLevel.WARN),
    name: "slides-rust-migration",
    resourcePersistencePath: join(temporary, "durable"),
    modulesRoot: join(root, "rust-worker/build"),
    modules: [
      { type: "ESModule", path: join(root, "rust-worker/build/index.js") },
      { type: "CompiledWasm", path: join(root, "rust-worker/build/index_bg.wasm") },
    ],
    compatibilityDate: "2026-10-12",
    bindings: {
      TELEGRAM_BOT_TOKEN: "local-test-token",
      TELEGRAM_WEBHOOK_SECRET: "local-test-secret",
      ALLOWED_TELEGRAM_USER_IDS: "7, 8",
    },
    durableObjects: { CONVERSION_JOBS: { className: "ConversionJob", useSQLite: true } },
    outboundService: async (request) => {
      const url = new URL(request.url);
      assert.equal(url.host, "api.telegram.org", "No real network calls are allowed");
      if (url.pathname.endsWith("/getFile")) {
        const { file_id } = await request.json();
        if (file_id === "missing") return Response.json({ ok: false });
        return Response.json({ ok: true, result: { file_path: `docs/${file_id}.pdf` } });
      }
      if (url.pathname.includes("/file/")) {
        downloads.push(url.pathname);
        if (blockDownload) await blockDownload;
        if (url.pathname.endsWith("/broken.pdf")) return new Response("not a pdf");
        if (url.pathname.endsWith("/oversized.pdf")) return new Response(new Uint8Array(20_000_001));
        return new Response(input, { headers: { "content-type": "application/pdf" } });
      }
      if (url.pathname.endsWith("/sendDocument")) {
        const form = await request.formData();
        const file = form.get("document");
        documents.push({
          chat: form.get("chat_id"), reply: JSON.parse(form.get("reply_parameters")),
          name: file.name, type: file.type, bytes: Buffer.from(await file.arrayBuffer()),
        });
        if (blockUpload) await blockUpload;
        return uploadFails ? new Response("uncertain upload", { status: 502 }) : Response.json({ ok: true, result: {} });
      }
      if (url.pathname.endsWith("/sendMessage")) {
        messages.push(await request.json());
        return Response.json({ ok: true, result: {} });
      }
      throw new Error(`Unexpected mock endpoint ${url.pathname}`);
    },
  }));
}

function update(messageId, overrides = {}) {
  return {
    update_id: messageId,
    message: {
      message_id: messageId, chat: { id: 42 }, from: { id: 7 },
      document: { file_id: "valid", file_name: "lecture.pdf", file_size: input.length, mime_type: "application/pdf", ...overrides },
    },
  };
}

async function submit(runtime, body, secret = "local-test-secret") {
  return runtime.dispatchFetch("https://local/telegram", {
    method: "POST", headers: { "X-Telegram-Bot-Api-Secret-Token": secret, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

async function waitFor(predicate) {
  const deadline = Date.now() + 10000;
  while (!predicate()) {
    assert.ok(Date.now() < deadline, "Timed out waiting for background job");
    await delay(20);
  }
}

test("Rust Worker migration in the real local workerd runtime", async (t) => {
  let runtime = createRuntime();
  try {
    await t.test("health, routing, secret, allowlist and malformed updates", async () => {
      const health = await runtime.dispatchFetch("https://local/health");
      assert.equal(health.status, 200);
      assert.equal((await health.json()).runtime, "rust-wasm");
      assert.equal((await runtime.dispatchFetch("https://local/other")).status, 404);
      assert.equal((await submit(runtime, update(1), "wrong")).status, 403);
      const denied = update(2);
      denied.message.from.id = 999;
      assert.equal((await submit(runtime, denied)).status, 200);
      assert.equal((await submit(runtime, { message: { text: "hello" } })).status, 200);
      const malformed = await runtime.dispatchFetch("https://local/telegram", {
        method: "POST", headers: { "X-Telegram-Bot-Api-Secret-Token": "local-test-secret" }, body: "{bad",
      });
      assert.equal(malformed.status, 400);
      assert.equal(downloads.length, 0);
    });

    await t.test("acknowledge while a download is blocked, collapse concurrent retries, and upload a valid PDF", async () => {
      blockDownload = new Promise((resolve) => { releaseDownload = resolve; });
      const response = await submit(runtime, update(10));
      assert.equal(response.status, 200);
      await waitFor(() => downloads.length === 1);
      const duplicates = await Promise.all(Array.from({ length: 20 }, () => submit(runtime, update(10))));
      assert.ok(duplicates.every((r) => r.status === 200));
      assert.equal(documents.length, 0, "Webhook must return before conversion finishes");
      releaseDownload();
      blockDownload = null;
      await waitFor(() => documents.length === 1);
      const document = documents[0];
      assert.equal(document.name, "lecture-notes.pdf");
      assert.equal(document.type, "application/pdf");
      assert.equal(document.chat, "42");
      assert.equal(document.reply.message_id, 10);
      await writeFile(join(temporary, "uploaded.pdf"), document.bytes);
      const check = spawnSync("uv", ["run", "python", "-c",
        "from pypdf import PdfReader; import sys; p=PdfReader(sys.argv[1]); assert len(p.pages)==2; assert 'Slide 4' in p.pages[1].extract_text()", join(temporary, "uploaded.pdf"),
      ], { cwd: root, encoding: "utf8" });
      assert.equal(check.status, 0, check.stderr);
      assert.equal(downloads.length, 1);
    });

    await t.test("deduplication survives a runtime restart", async () => {
      await runtime.dispose();
      runtime = createRuntime();
      assert.equal((await submit(runtime, update(10))).status, 200);
      await delay(250);
      assert.equal(documents.length, 1);
      assert.equal(downloads.length, 1);
    });

    await t.test("invalid file type and declared size produce one reply each without downloading", async () => {
      const count = messages.length;
      await submit(runtime, update(20, { file_name: "text.txt", mime_type: "text/plain" }));
      await submit(runtime, update(21, { file_size: 20_000_001 }));
      await waitFor(() => messages.length === count + 2);
      assert.equal(downloads.length, 1);
      assert.ok(messages.slice(count).every((m) => m.text.includes("20 MB")));
    });

    await t.test("download failures, corrupt PDFs and untruthful file sizes fail once", async () => {
      const count = messages.length;
      for (const [index, file_id] of ["missing", "broken", "oversized"].entries()) {
        await submit(runtime, update(30 + index, { file_id }));
      }
      await waitFor(() => messages.length === count + 3);
      assert.equal(documents.length, 1);
    });

    await t.test("an uncertain upload is never sent again on webhook redelivery", async () => {
      const count = messages.length;
      uploadFails = true;
      await submit(runtime, update(40));
      await waitFor(() => documents.length === 2 && messages.length === count + 1);
      await submit(runtime, update(40));
      await delay(250);
      assert.equal(documents.length, 2);
      uploadFails = false;
    });

    await t.test("distinct jobs queue without overlapping downloads", async () => {
      const beforeDownloads = downloads.length;
      const beforeDocuments = documents.length;
      blockDownload = new Promise((resolve) => { releaseDownload = resolve; });
      await submit(runtime, update(50));
      await waitFor(() => downloads.length === beforeDownloads + 1);
      assert.equal((await submit(runtime, update(51))).status, 200);
      await delay(150);
      assert.equal(downloads.length, beforeDownloads + 1);
      releaseDownload();
      blockDownload = null;
      await waitFor(() => documents.length === beforeDocuments + 2);
      assert.deepEqual(documents.slice(beforeDocuments).map((d) => d.reply.message_id), [50, 51]);
    });

    await t.test("a process crash during an upload leaves a durable no-resend marker", async () => {
      const before = documents.length;
      blockUpload = new Promise((resolve) => { releaseUpload = resolve; });
      await submit(runtime, update(60));
      await waitFor(() => documents.length === before + 1);
      const stopping = runtime.dispose();
      await delay(100);
      releaseUpload();
      blockUpload = null;
      await stopping;
      const files = await readdir(join(temporary, "durable"), { recursive: true });
      const database = files.find((p) => p.endsWith(".sqlite") && !p.endsWith("metadata.sqlite"));
      assert.ok(database);
      const inspection = spawnSync("uv", ["run", "python", "-c",
        "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(c.execute(\"SELECT status FROM job WHERE id='42:60'\").fetchone()[0])",
        join(temporary, "durable", database),
      ], { cwd: root, encoding: "utf8" });
      assert.equal(inspection.status, 0, inspection.stderr);
      assert.equal(inspection.stdout.trim(), "sending", "The test must actually interrupt an unfinished upload");
      runtime = createRuntime();
      await submit(runtime, update(60));
      await submit(runtime, update(61));
      await waitFor(() => documents.length === before + 2);
      await delay(200);
      assert.deepEqual(documents.slice(before).map((d) => d.reply.message_id), [60, 61]);
    });
  } finally {
    await runtime.dispose();
    await rm(temporary, { recursive: true, force: true });
  }
});
