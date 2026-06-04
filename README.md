# pre-commit-search-and-replace

This is a plugin for [pre-commit](https://pre-commit.com) that will run search and replace string operations against files to be committed.
Note that this plugin operates on each line of a file therefore multiple line patterns are not supported.

This fork of the [Ruby original](https://github.com/mattlqx/pre-commit-search-and-replace) is written in Rust for fast, zero-runtime-dependency execution.

## Installation

### As a pre-commit hook

Add to your `.pre-commit-config.yaml`:

    - repo: https://github.com/cbachhuber/pre-commit-search-and-replace
      rev: v1.1.2
      hooks:
      - id: search-and-replace

pre-commit will build the Rust binary automatically via Cargo.

### Standalone

Build from source with [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

    cargo install --path .

Or build manually:

    cargo build --release
    # Binary is at target/release/search-and-replace

## Performance Benchmarks

This repository includes two benchmark workflows:

- Rust microbenchmarks (`cargo bench`) for core parsing/search paths.
- Cold-start CLI comparison (`hyperfine`) against the upstream Ruby implementation.

### 1) Rust Microbenchmarks

Run:

  cargo bench

This executes Criterion benchmarks in `benches/performance.rs`, including:

- Literal vs regex pattern parsing
- Fixed fixture parsing scenarios
- File-size scaling tests

### 2) Cold-Start CLI Comparison (Rust vs Ruby Upstream)

Prerequisites:

- `hyperfine` installed
- Rust binary built: `cargo build --release`
- Optional Ruby baseline:
  - Local clone of `https://github.com/mattlqx/pre-commit-search-and-replace`
  - `bundle install` run in that clone

Run Rust-only cold-start benchmark:

  python3 scripts/benchmark_cold_start.py

Run Rust vs Ruby comparison:

  python3 scripts/benchmark_cold_start.py --ruby-repo /path/to/pre-commit-search-and-replace

Results are written to:

  benchmark/results/cold_start.json

Generate a markdown comparison table from the JSON output:

  python3 scripts/benchmark_results_to_markdown.py

Write the table to a file instead of stdout:

  python3 scripts/benchmark_results_to_markdown.py --output benchmark/results/cold_start.md

### GitHub Action (Automated Setup)

If you prefer not to install Rust, Ruby, Bundler, and hyperfine locally, use the
manual benchmark workflow in [benchmark workflow](.github/workflows/benchmark.yml).

From GitHub:

- Go to Actions -> Benchmark -> Run workflow
- Optionally set `warmup`, `min_runs`, and `ruby_ref`

The workflow will:

- Build the Rust release binary
- Clone and set up the upstream Ruby implementation
- Run the cold-start comparison with hyperfine
- Generate `cold_start.json` and `cold_start.md`
- Upload results as workflow artifacts and add the markdown table to the job summary

### Benchmark Notes

- These benchmarks are manual by design and are not PR-blocking CI checks.
- Cold-start numbers vary by machine, filesystem state, and background load.
- Use the same hardware and similar system load when comparing before/after changes.
- Ratios in the markdown report are computed against the selected baseline tool (default: `rust`).

## Usage

By default, a YAML config file is loaded at `.pre-commit-search-and-replace.yaml` in the root of the repo. This config file should be a list of entries specifying any of the following keys:

- `search`: (required) the string or regexp to search for. To use a regexp, start and end with slashes. e.g. `/^mypattern/`
- `replacement`: the string to replace matched strings with. If specified, files will "fixed". Match groups can be referenced here (e.g. `\1` or `\k<foo>`)
- `insensitive`: boolean whether the regexp should be case-insensitive. default: `false`
- `extended`: boolean whether the regexp should be extended. default: `false`
- `description`: short text description of purpose of the entry.

The config file name can be changed by passing a `-c`/`--config PATH` argument to the hook in the pre-commit config. A single search and replacement can be specified with `-s`/`--search STRING` and `-r`/`--replacement STRING` arguments as well instead of using a config file.

Other command line args:

- `-i`/`--insensitive` - Case-insensitive search.
- `-e`/`--extended` - Extended regexp search.
- `-w`/`--write`, `--no-write` - Whether to write replacements to the file. Default: true.
- `-C`/`--color`, `--no-color` - Whether to have output be colorized. Default: true.

The `NO_COLOR` environment variable is also respected to disable colored output.

Example pre-commit config:

    - repo: https://github.com/cbachhuber/pre-commit-search-and-replace
      rev: v1.1.2
      hooks:
      - id: search-and-replace

Example search-and-replace config:

    - search: /Something [bB]ad/
      replacement: Something Good
    - search: foobar
      insensitive: true
      replacement: FOOBAR
    - search: JustFailIfThisStringIsFound

Output can look something like this:

![example output](doc/example.png)

Specific lines in the committed files may be exempt from consideration by commenting as appropriate to the end of the line:

```
# no-search-replace

// no-search-replace
```

Or these comment styles that support being anywhere in the line:

```
/* no-search-replace */

<!-- no-search-replace -->
```
