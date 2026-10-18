// Includes local mocked getFile/download/multipart upload overhead. No external requests.
import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { setTimeout as delay } from "node:timers/promises";
import { performance } from "node:perf_hooks";
import { Miniflare, Log, LogLevel, convertV4MiniflareOptions } from "miniflare";

const root = resolve(import.meta.dirname, "..");
const fixtures = resolve(process.argv[2]);
const rows = JSON.parse(await readFile(join(fixtures, "results.json"), "utf8"));
const temporary = await mkdtemp(join(tmpdir(), "slides-wasm-benchmark-"));
const completed = [];
let input;
let start;
const runtime = new Miniflare(convertV4MiniflareOptions({
  name: "slides-rust-benchmark", log: new Log(LogLevel.WARN),
  resourcePersistencePath: temporary,
  modulesRoot: join(root, "rust-worker/build"),
  modules: [
    { type: "ESModule", path: join(root, "rust-worker/build/index.js") },
    { type: "CompiledWasm", path: join(root, "rust-worker/build/index_bg.wasm") },
  ],
  compatibilityDate: "2026-10-12",
  bindings: { TELEGRAM_BOT_TOKEN: "local", TELEGRAM_WEBHOOK_SECRET: "local", ALLOWED_TELEGRAM_USER_IDS: "7" },
  durableObjects: { CONVERSION_JOBS: { className: "ConversionJob", useSQLite: true } },
  outboundService: async (request) => {
    const url = new URL(request.url);
    assert.equal(url.host, "api.telegram.org");
    if (url.pathname.endsWith("/getFile")) {
      start = performance.now();
      return Response.json({ ok: true, result: { file_path: "fixture.pdf" } });
    }
    if (url.pathname.includes("/file/")) return new Response(input);
    if (url.pathname.endsWith("/sendDocument")) {
      const form = await request.formData();
      const bytes = Buffer.from(await form.get("document").arrayBuffer());
      completed.push({ milliseconds: performance.now() - start, bytes: bytes.length });
      return Response.json({ ok: true, result: {} });
    }
    throw new Error("Benchmark conversion failed or unexpected Telegram call");
  },
}));

try {
  let message = 1;
  for (const row of rows) {
    input = await readFile(join(fixtures, `${row.name}.pdf`));
    const times = [];
    for (let repeat = 0; repeat < 3; repeat++) {
      const count = completed.length;
      const response = await runtime.dispatchFetch("https://local/telegram", {
        method: "POST", headers: { "X-Telegram-Bot-Api-Secret-Token": "local" },
        body: JSON.stringify({ message: {
          message_id: message++, chat: { id: 42 }, from: { id: 7 },
          document: { file_id: "fixture", file_name: `${row.name}.pdf`, file_size: input.length, mime_type: "application/pdf" },
        } }),
      });
      assert.equal(response.status, 200);
      const deadline = Date.now() + 60000;
      while (completed.length === count) {
        assert.ok(Date.now() < deadline, `Timeout for ${row.name}`);
        await delay(20);
      }
      assert.equal(completed.at(-1).bytes, row.rust_bytes);
      times.push(completed.at(-1).milliseconds);
    }
    row.rust_wasm_pipeline_ms = Math.round(times.sort((a, b) => a-b)[1] * 100) / 100;
    row.wasm_speedup = Math.round(row.python_ms / row.rust_wasm_pipeline_ms * 100) / 100;
    console.log(JSON.stringify(row));
  }
  await writeFile(join(fixtures, "results.json"), JSON.stringify(rows, null, 2) + "\n");
} finally {
  await runtime.dispose();
  await rm(temporary, { recursive: true, force: true });
}
