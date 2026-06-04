# Contributing Guidelines

## Code of Conduct

This project is intended to be a safe, welcoming space for collaboration. All contributors are expected to adhere to the [Contributor Covenant](http://contributor-covenant.org) code of conduct. Thank you for being kind to each other!

## Contributions welcome!

**Before spending lots of time on something, ask for feedback on your idea first!**

Please search [issues](../../issues/) and [pull requests](../../pulls/) before adding something new! This helps avoid duplicating efforts and conversations.

This project welcomes any kind of contribution! Here are a few suggestions:

- **Ideas**: participate in an issue thread or start your own to have your voice heard.
- **Writing**: contribute your expertise in an area by helping expand the included content.
- **Copy editing**: fix typos, clarify language, and generally improve the quality of the content.
- **Formatting**: help keep content easy to read with consistent formatting.
- **Code**: help maintain and improve the project codebase.

## Code Style

Use `cargo fmt` to format code and `cargo clippy` for linting. A `rustfmt` pre-commit hook is included to enforce formatting automatically.

## Running Tests

Run the full test suite with:

```sh
cargo test
```

This executes both unit tests (in `src/lib.rs`) and integration tests (in `tests/integration_test.rs`). All tests must pass before submitting a pull request.

## Running Benchmarks

Performance benchmarks are manual and intended for local regression checks.

Run Rust microbenchmarks:

```sh
cargo bench
```

Run cold-start benchmark for the local Rust binary:

```sh
cargo build --release
python3 scripts/benchmark_cold_start.py
```

Run Rust vs upstream Ruby comparison:

```sh
python3 scripts/benchmark_cold_start.py --ruby-repo /path/to/pre-commit-search-and-replace
```

Generate a markdown summary table from benchmark JSON:

```sh
python3 scripts/benchmark_results_to_markdown.py
```

To run the full Rust-vs-Ruby benchmark without local setup, use the manual workflow in
`.github/workflows/benchmark.yml` from the GitHub Actions UI.

The cold-start benchmark requires `hyperfine`. Results are written to `benchmark/results/cold_start.json`.

## Project Governance

**This is an [OPEN Open Source Project](http://openopensource.org/).**

Individuals making significant and valuable contributions are given commit access to the project to contribute as they see fit. This project is more like an open wiki than a standard guarded open source project.

### Rules

There are a few basic ground rules for collaborators:

1. **No `--force` pushes** or modifying the Git history in any way.
1. **Non-master branches** ought to be used for ongoing work.
1. Contributors should attempt to adhere to the prevailing code style.

### Releases

Declaring formal releases remains the prerogative of the project maintainer.

### Changes to this arrangement

This is an experiment and feedback is welcome! This document may also be subject to pull requests or changes by contributors where you believe you have something valuable to add or change.
