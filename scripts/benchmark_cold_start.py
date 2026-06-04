#!/usr/bin/env python3
"""Cold-start CLI benchmark for search-and-replace.

Compares this Rust implementation against the upstream Ruby implementation from:
https://github.com/mattlqx/pre-commit-search-and-replace

The benchmark uses hyperfine and writes JSON results to benchmark/results.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


def check_hyperfine() -> bool:
    try:
        subprocess.run(["hyperfine", "--version"], check=True, capture_output=True)
        return True
    except (FileNotFoundError, subprocess.CalledProcessError):
        print("error: hyperfine not found")
        print("install with one of:")
        print("  - Ubuntu/Debian: sudo apt install hyperfine")
        print("  - Fedora: sudo dnf install hyperfine")
        print("  - macOS: brew install hyperfine")
        return False


def rust_tool(project_root: Path, fixture_file: Path, fixture_config: Path) -> dict | None:
    binary = project_root / "target" / "release" / "search-and-replace"
    if not binary.exists():
        print("warning: Rust binary not found; skipping rust tool")
        print("  build with: cargo build --release")
        return None

    cmd = (
        f"'{binary}' --no-color --no-write -c '{fixture_config}' '{fixture_file}'"
    )
    return {"name": "rust", "cmd": cmd}


def ruby_tool(
    ruby_repo: Path | None,
    fixture_file: Path,
    fixture_config: Path,
) -> dict | None:
    if ruby_repo is None:
        print("warning: --ruby-repo not provided; skipping ruby baseline")
        return None

    if not ruby_repo.exists():
        print(f"warning: ruby repo path does not exist: {ruby_repo}")
        return None

    bin_path = ruby_repo / "bin" / "search-and-replace"
    if not bin_path.exists():
        print(f"warning: ruby search-and-replace executable not found at {bin_path}")
        return None

    if shutil.which("bundle") is None:
        print("warning: bundle not found; skipping ruby baseline")
        print("  install bundler and run `bundle install` in the ruby repo")
        return None

    cmd = (
        f"cd '{ruby_repo}' && bundle exec ./bin/search-and-replace "
        f"--no-color --no-write -c '{fixture_config}' '{fixture_file}'"
    )
    return {"name": "ruby-upstream", "cmd": cmd}


def run_benchmark(tools: list[dict], warmup: int, min_runs: int, output_json: Path) -> int:
    if not tools:
        print("error: no benchmarkable tools discovered")
        return 1

    output_json.parent.mkdir(parents=True, exist_ok=True)

    args: list[str] = [
        "hyperfine",
        "--warmup",
        str(warmup),
        "--min-runs",
        str(min_runs),
        "--prepare",
        "sync",
        "--ignore-failure",
        "--export-json",
        str(output_json),
        "--style",
        "full",
    ]

    for tool in tools:
        args.extend(["--command-name", tool["name"], tool["cmd"]])

    print("running hyperfine with tools:")
    for tool in tools:
        print(f"  - {tool['name']}")

    completed = subprocess.run(args)
    if completed.returncode != 0:
        print("error: hyperfine benchmark failed")
        return completed.returncode

    print(f"saved benchmark results to {output_json}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run cold-start CLI benchmark for Rust and Ruby search-and-replace",
    )
    parser.add_argument(
        "--ruby-repo",
        type=Path,
        default=None,
        help="Path to local clone of mattlqx/pre-commit-search-and-replace",
    )
    parser.add_argument(
        "--fixture-file",
        type=Path,
        default=Path("tests/fixtures/bad_content.txt"),
        help="File to pass to both tools",
    )
    parser.add_argument(
        "--fixture-config",
        type=Path,
        default=Path("tests/fixtures/search-and-replace.yaml"),
        help="Config passed to both tools",
    )
    parser.add_argument("--warmup", type=int, default=2, help="Hyperfine warmup runs")
    parser.add_argument("--min-runs", type=int, default=5, help="Hyperfine min runs")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("benchmark/results/cold_start.json"),
        help="Where to write hyperfine JSON output",
    )
    args = parser.parse_args()

    project_root = Path(__file__).resolve().parent.parent
    fixture_file = (project_root / args.fixture_file).resolve()
    fixture_config = (project_root / args.fixture_config).resolve()

    if not fixture_file.exists():
        print(f"error: fixture file not found: {fixture_file}")
        return 1
    if not fixture_config.exists():
        print(f"error: fixture config not found: {fixture_config}")
        return 1

    if not check_hyperfine():
        return 1

    tools: list[dict] = []

    rust = rust_tool(project_root, fixture_file, fixture_config)
    if rust is not None:
        tools.append(rust)

    ruby = ruby_tool(args.ruby_repo.resolve() if args.ruby_repo else None, fixture_file, fixture_config)
    if ruby is not None:
        tools.append(ruby)

    return run_benchmark(tools, args.warmup, args.min_runs, (project_root / args.output).resolve())


if __name__ == "__main__":
    sys.exit(main())
