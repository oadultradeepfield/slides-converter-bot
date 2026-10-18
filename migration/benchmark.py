"""Temporary comparison benchmark; generated decks remain outside the repository."""

import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

from migration.test_parity import BINARY, create_deck
from slides_converter_bot.converter import convert_pdf

CASES = [
    ("30-slides", 30, False),
    ("100-slides", 100, False),
    ("300-slides", 300, False),
    ("30-images", 30, True),
    ("200-images", 200, True),
]
REPEATS = 3


def benchmark(directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    results: list[dict[str, str | int | float]] = []

    for name, pages, images in CASES:
        source = directory / f"{name}.pdf"
        create_deck(source, pages, images=images)
        python_times: list[float] = []
        rust_times: list[float] = []
        python_output = directory / f"{name}-python.pdf"
        rust_output = directory / f"{name}-rust.pdf"

        for _ in range(REPEATS):
            start = time.perf_counter()
            convert_pdf(source, python_output)
            python_times.append((time.perf_counter() - start) * 1000)
            result = subprocess.run(
                [str(BINARY), str(source), str(rust_output)],
                capture_output=True,
                text=True,
                check=True,
            )
            rust_times.append(
                float(result.stderr.split("conversion_ms=")[1].split()[0])
            )

        row: dict[str, str | int | float] = {
            "name": name,
            "pages": pages,
            "input_bytes": source.stat().st_size,
            "python_ms": round(statistics.median(python_times), 2),
            "rust_native_ms": round(statistics.median(rust_times), 2),
            "python_bytes": python_output.stat().st_size,
            "rust_bytes": rust_output.stat().st_size,
        }
        results.append(row)
        print(json.dumps(row), flush=True)

    (directory / "results.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    benchmark(Path(sys.argv[1]))
