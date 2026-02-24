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
    *   `ranim/benches/`: Benchmark definitions (`eval`, `render`, `extract`).
    *   `ranim/justfile`: Task runner configuration for `ranim`.
*   `db/`: Storage for benchmark results.
    *   Structure: `db/<commit_hash>/<run_name>/...`
    *   `db/db.json`: Global manifest mapping commit hashes to run names.
    *   Each run directory contains `run.json` (system info + benchmark IDs) and per-benchmark JSON files.
*   `web/`: Web frontend for visualizing benchmark results.

## Benchmark Suite

There are 22 benchmarks across 3 groups:

| Group | Benchmark | Parameters |
|-------|-----------|------------|
| eval | eval_static_squares | 10, 100, 1000 |
| eval | eval_transform_squares | 10, 100, 1000 |
| extract | polygon | 5, 10, 20, 40 |
| extract | square, rectangle, circle, arc | (none) |
| render | static_squares | 5, 10, 20, 40 |
| render | transform_squares | 5, 10, 20, 40 |

The `render` group benchmarks are the most time-consuming (especially `render/transform_squares/40`, which can take ~10 minutes alone). A full benchmark run takes approximately 20-30 minutes.

## Usage

### Running Benchmarks

The harness uses subcommands. To run benchmarks:

```bash
cargo run --release -- bench --name <run_name>
```

**Options:**

*   `--name <string>`: (Required) Name for this benchmark run (e.g., "macbookpro", "aorus").
*   `--allow-dirty`: Skip the check that ensures the `ranim` submodule has no uncommitted changes. **Required when the submodule is in detached HEAD state** (e.g., when checking out specific commits).
*   `--force`: Overwrite existing output in `db/` if the directory for this specific run already exists.

**Other subcommands:**

*   `cargo run --release -- graph`: Generate git-graph visualization and copy data for web frontend.
*   `cargo run --release -- sync`: Rebuild `db.json` and `run.json` manifests from existing benchmark data.

### Developing `ranim`

If you are working on the `ranim` engine itself (inside the `ranim/` directory), standard Rust workflows apply.

*   **Build**: `cargo build`
*   **Test**: `cargo test`
*   **Task Runner**: The project uses `just`.
    *   `just fmt`: Format code.
    *   `just lint`: Run clippy and build docs.
    *   `just clean`: Remove logs/artifacts.

## Benchmark Workflow for Agents

When asked to run benchmarks for new/missing commits, follow this procedure:

### 1. Fetch latest commits

```bash
cd ranim && git fetch origin
```

### 2. Identify which commits need benchmarking

Only benchmark commits that were **squash-merged via PR** (commit message ends with `(#NNN)`):

```bash
cd ranim && git log origin/main --oneline | grep -E '\(#[0-9]+\)$'
```

### 3. Check what's already been benchmarked

```bash
cat db/db.json
```

Compare the commit hashes from step 2 against `db/db.json` to find commits missing the current machine's run name.

### 4. Run benchmarks for each missing commit

For each commit that needs benchmarking, from oldest to newest:

```bash
# Checkout the commit in the submodule
cd ranim && git checkout <commit_hash>

# Run the benchmark (from the repo root)
cargo run --release -- bench --name <machine_name> --allow-dirty
```

**Important notes:**
*   Use `--allow-dirty` because checking out specific commits puts the submodule in detached HEAD state.
*   Use `--force` only if you need to overwrite an existing run for the same commit+name.
*   Each run takes ~20-30 minutes. Set a generous timeout (at least 3600 seconds).
*   The harness will compile the benchmarks for each commit (may take 1-2 minutes on first build, ~10 seconds for incremental).

### 5. Restore the submodule

After all benchmarks are done, checkout the latest benchmarked commit:

```bash
cd ranim && git checkout <latest_commit_hash>
```

### Batch execution example

To run multiple commits in sequence:

```bash
for commit in <hash1> <hash2> <hash3>; do
    git -C ranim checkout $commit 2>&1
    cargo run --release -- bench --name macbookpro --allow-dirty 2>&1
done
```

## Key Configuration

*   **Cargo.toml**: Dependencies for the harness (clap, sysinfo, criterion, wgpu, git-graph).
*   **ranim/benches/Cargo.toml**: Defines the `eval`, `render`, and `extract` benchmark targets (all with `harness = false`, using Criterion directly).

## Notes

*   The harness requires the `ranim` submodule to be initialized (`git submodule update --init --recursive`).
*   Output logs for the harness use `tracing` and print to stdout/stderr.
*   The `render` benchmarks require GPU access (Metal on macOS, Vulkan on Windows/Linux).
*   Benchmark results include `slope`, `mean`, `median`, `median_abs_dev` (in nanoseconds), `measured_values`, `iteration_count`, and `change` vs. previous run.
