# Temporary Rust migration checks

This directory retains the migration tests and benchmark commands so the result
can be reproduced before a production cutover. The existing Python converter is
the comparison oracle. Remove this directory and the legacy implementation once
the Rust version has been validated with representative real inputs and the
rollback window has ended.

## Checks

```sh
npm run migration:test
```

`test_parity.py` builds generated PDFs, runs both converters, and compares page
counts, A4 dimensions, extracted text, annotation counts and rendered pixels.
It also covers rotation, inherited resources and page origins, embedded Unicode
fonts, crop boxes, multiple content streams, URI links, direct/named internal
links, encrypted inputs, malformed compression and input/decompression limits.

`test_worker.mjs` loads the compiled Wasm in Cloudflare's local `workerd` runtime.
Every Telegram request is intercepted by a local mock. It checks HTTP routing,
authentication, sender authorization, immediate acknowledgement while a download
is blocked, concurrent duplicate deliveries, persistent deduplication after a
restart, bounded downloads, error replies, FIFO queue serialization and an
actual runtime interruption during an upload. The interruption check reads the
persisted SQLite marker before restarting and verifies that the document is not
uploaded again.

Prerequisites are listed in [`rust-worker/README.md`](../rust-worker/README.md).
PDF comparisons use Poppler's `pdftoppm`, with a maximum mean channel difference
of 0.5/255 at a 1200-pixel render size. Small decks are rendered in full; larger
decks compare the first and last output page and check text/dimensions on every
page. Rotations use a normalized reference so correctness is checked against the
source orientation rather than the old converter's rotation handling.

## Benchmarks

```sh
uv run python -m migration.benchmark /private/tmp/slides-rust-benchmark
node migration/benchmark_worker.mjs /private/tmp/slides-rust-benchmark
```

These commands generate deterministic decks and take the median of three runs.
The Python number measures conversion, and the native Rust number measures the
same conversion interface. The Wasm number runs the compiled Worker and includes
local mocked Telegram file lookup, download and multipart upload handling. It
excludes the queue's alarm delay, runtime startup and actual Internet transfer
time. These generated fixtures establish a local comparison, not a production
latency guarantee.

The implementation improves both language execution and the algorithm: it
embeds original slide streams as PDF Form XObjects, draws one shared notes grid,
keeps shared source resources, and resolves named link destinations once per
document. It avoids Python's repeated stream parsing and per-page grid creation.

Recorded results are in [`results.json`](results.json).

Measured locally on 18 October 2026 after the module refactor:

| Generated input | Python conversion | Rust/Wasm pipeline |
| --- | ---: | ---: |
| 30 vector slides | 859 ms | 8 ms |
| 100 vector slides | 2,942 ms | 23 ms |
| 300 vector slides | 9,378 ms | 39 ms |
| 30 image slides, 2.6 MB | 1,091 ms | 31 ms |
| 200 image slides, 17.4 MB | 7,783 ms | 100 ms |

Validation: 23 generated-PDF cases, 8 Worker integration scenarios (plus the
parent test), the existing Python tests, Rust Clippy for both targets, and a
Wrangler deployment dry run. No live deployment or webhook switch was performed
as part of these checks.
