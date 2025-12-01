# ranim-bench & ranim

## Project Overview

This workspace contains **ranim-bench**, a custom benchmarking harness designed for the **ranim** animation engine.

*   **ranim-bench**: The root project. It is a CLI tool (`src/main.rs`) that automates the execution of benchmarks. It handles:
    *   Verifying the git state of the `ranim` submodule.
    *   Collecting system information (CPU, Memory, GPU).
    *   Invoking `cargo criterion` within the `ranim/benches` directory.
    *   Structuring and saving output data (JSON) to the `db/` directory, organized by commit hash and run name.
*   **ranim** (Submodule): A Rust animation engine inspired by Manim. It resides in the `ranim/` directory and contains the core logic and the actual benchmark implementations (`ranim/benches/`).

## Directory Structure

*   `src/`: Source code for the `ranim-bench` harness.
*   `ranim/`: The `ranim` source code (submodule).
    *   `ranim/benches/`: detailed benchmark definitions (`eval`, `render`, `extract`).
    *   `ranim/justfile`: Task runner configuration for `ranim`.
*   `db/`: Storage for benchmark results.
    *   Structure: `db/<commit_hash>/<run_name>/...`

## Usage

### Running Benchmarks

To run the benchmarks, use `cargo run` from the root directory. You must provide a name for the benchmark run.

```bash
cargo run -- --name <run_name>
```

**Options:**

*   `--name <string>`: (Required) Name for this benchmark run (e.g., "macbookpro", "test-run").
*   `--allow-dirty`: Skip the check that ensures the `ranim` submodule has no uncommitted changes.
*   `--force`: Overwrite existing output in `db/` if the directory for this specific run already exists.

**Example:**

```bash
cargo run -- --name my-local-bench --allow-dirty
```

### Developing `ranim`

If you are working on the `ranim` engine itself (inside the `ranim/` directory), standard Rust workflows apply.

*   **Build**: `cargo build`
*   **Test**: `cargo test`
*   **Task Runner**: The project uses `just`.
    *   `just fmt`: Format code.
    *   `just lint`: Run clippy and build docs.
    *   `just clean`: Remove logs/artifacts.

## Key Configuration

*   **ranim-bench/Cargo.toml**: Dependencies for the harness (clap, sysinfo, criterion, wgpu).
*   **ranim/benches/Cargo.toml**: Defines the `eval`, `render`, and `extract` benchmarks and depends on the local `ranim` crate.

## Notes

*   The harness requires the `ranim` submodule to be initialized (`git submodule update --init --recursive`).
*   Output logs for the harness use `tracing` and print to stdout/stderr.
