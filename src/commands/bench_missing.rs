use crate::utils::run_git;
use anyhow::{Context, Result};
use indicatif::ProgressStyle;
use std::path::Path;
use tracing::{info, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Find PR-merged commits on origin/main that are missing benchmarks for the given machine name,
/// then run benchmarks for each one.
pub fn run(repo_dir: &Path, name: &str, force: bool, dry_run: bool) -> Result<()> {
    let root_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_root = root_dir.join("db");

    // 1. Fetch latest
    info!("Fetching origin...");
    run_git(repo_dir, ["fetch", "origin"])?;

    // 2. Get PR-merged commits (message ends with (#NNN))
    let log_output = run_git(
        repo_dir,
        ["log", "origin/main", "--oneline", "--format=%H %s"],
    )?;
    let pr_commits: Vec<(String, String)> = log_output
        .lines()
        .filter_map(|line| {
            let (hash, msg) = line.split_once(' ')?;
            // Check if message ends with (#NNN)
            if msg.ends_with(')')
                && msg.rfind("(#").is_some_and(|i| {
                    msg[i + 2..msg.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_digit())
                })
            {
                Some((hash.to_string(), msg.to_string()))
            } else {
                None
            }
        })
        .collect();

    // 3. Find which commits are missing benchmarks for this machine
    let missing: Vec<&(String, String)> = pr_commits
        .iter()
        .filter(|(hash, _)| {
            let run_dir = db_root.join(hash).join(name);
            if force {
                true
            } else {
                !run_dir.exists()
            }
        })
        .collect();

    if missing.is_empty() {
        info!(
            "All PR commits already have '{}' benchmarks. Nothing to do.",
            name
        );
        return Ok(());
    }

    // Show what we'll run (oldest first)
    info!(
        "Found {} commits missing '{}' benchmarks:",
        missing.len(),
        name
    );
    for (hash, msg) in missing.iter().rev() {
        info!("  {} {}", &hash[..8], msg);
    }

    if dry_run {
        info!("Dry run — not running benchmarks.");
        return Ok(());
    }

    // 4. Run benchmarks oldest-first
    let to_run: Vec<_> = missing.into_iter().rev().cloned().collect();
    let total = to_run.len();

    let batch_span = tracing::info_span!("bench-missing");
    batch_span.pb_set_style(
        &ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    batch_span.pb_set_length(total as u64);
    let _batch_guard = batch_span.enter();

    for (hash, msg) in to_run.iter() {
        batch_span.pb_set_message(&format!("{} {}", &hash[..8], msg));

        // Checkout
        run_git(repo_dir, ["checkout", hash])
            .with_context(|| format!("Failed to checkout {}", hash))?;

        // Run benchmark
        match crate::commands::bench::run(repo_dir, name, force) {
            Ok(()) => info!("Completed benchmark for {}", &hash[..8]),
            Err(e) => {
                warn!("Benchmark failed for {}: {}", &hash[..8], e);
                warn!("Continuing with next commit...");
            }
        }

        batch_span.pb_inc(1);
    }

    // 5. Restore to latest benchmarked commit
    if let Some((latest_hash, _)) = to_run.last() {
        info!("Restoring submodule to {}", &latest_hash[..8]);
        run_git(repo_dir, ["checkout", latest_hash])?;
    }

    info!("Done! Benchmarked {} commits.", total);
    Ok(())
}
