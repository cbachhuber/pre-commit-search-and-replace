#!/usr/bin/env python3
"""Convert hyperfine JSON benchmark output into a markdown comparison table.

Default input: benchmark/results/cold_start.json
Prints markdown to stdout, or writes to --output if provided.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def format_ms(seconds: float) -> str:
    return f"{seconds * 1000.0:.2f} ms"


def build_table(results: list[dict], baseline: str) -> str:
    sorted_results = sorted(results, key=lambda item: item.get("mean", float("inf")))

    baseline_row = next((item for item in sorted_results if item.get("command") == baseline), None)
    baseline_mean = baseline_row.get("mean", 0.0) if baseline_row else 0.0

    lines = [
        "| Tool | Mean | Std Dev | vs baseline |",
        "| --- | ---: | ---: | ---: |",
    ]

    for item in sorted_results:
        name = str(item.get("command", "unknown"))
        mean = float(item.get("mean", 0.0))
        stddev = float(item.get("stddev", 0.0))

        if baseline_mean > 0.0:
            ratio = mean / baseline_mean
            ratio_text = f"{ratio:.2f}x"
        else:
            ratio_text = "n/a"

        lines.append(
            f"| {name} | {format_ms(mean)} | {format_ms(stddev)} | {ratio_text} |"
        )

    if baseline_row:
        lines.append("")
        lines.append(f"Baseline: {baseline}")
    else:
        lines.append("")
        lines.append(f"Baseline '{baseline}' not found in results.")

    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate a markdown benchmark table from hyperfine JSON output",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=Path("benchmark/results/cold_start.json"),
        help="Path to hyperfine JSON output",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Optional output file path for markdown table",
    )
    parser.add_argument(
        "--baseline",
        default="rust",
        help="Command name to use as baseline for ratio column",
    )
    args = parser.parse_args()

    in_path = args.input.resolve()
    if not in_path.exists():
        print(f"error: input JSON not found: {in_path}")
        return 1

    try:
        data = json.loads(in_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"error: failed to parse JSON from {in_path}: {exc}")
        return 1

    results = data.get("results")
    if not isinstance(results, list) or not results:
        print(f"error: no benchmark results found in {in_path}")
        return 1

    markdown = build_table(results, args.baseline)

    if args.output is None:
        sys.stdout.write(markdown)
    else:
        out_path = args.output.resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(markdown, encoding="utf-8")
        print(f"wrote markdown table to {out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
