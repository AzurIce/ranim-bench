mod commands {
    pub mod bench;
    pub mod bench_missing;
    pub mod graph;
    pub mod sync;
}
mod common;
mod utils;

use crate::utils::run_git;
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::level_filters::LevelFilter;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run benchmarks for a single commit
    Bench {
        /// Skip working directory clean check
        #[arg(long)]
        allow_dirty: bool,
        /// Machine/run name (e.g. "macbookpro", "aorus")
        #[arg(long)]
        name: String,
        /// Overwrite existing output directory
        #[arg(long)]
        force: bool,
    },
    /// Auto-benchmark all PR-merged commits missing data for this machine
    BenchMissing {
        /// Machine/run name (e.g. "macbookpro", "aorus")
        #[arg(long)]
        name: String,
        /// Overwrite existing benchmark data
        #[arg(long)]
        force: bool,
        /// Only show what would be benchmarked, don't run
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate git-graph and all-data.json for web
    Graph,
    /// Sync run.json files from db structure
    Sync,
}

fn main() -> Result<()> {
    init_tracing();

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_dir = root_dir.join("ranim");
    assert_submodule_initialized(&repo_dir)?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Bench {
            allow_dirty,
            name,
            force,
        } => {
            if !allow_dirty {
                ensure_clean(&repo_dir)?;
            } else {
                warn!("allow dirty is true, skipping clean check...");
            }

            info!("benchmarking run '{}'...", name);
            commands::bench::run(&repo_dir, &name, force)?;
        }
        Commands::Graph => commands::graph::run(&root_dir, &repo_dir)?,
        Commands::Sync => commands::sync::run(&root_dir)?,
        Commands::BenchMissing {
            name,
            force,
            dry_run,
        } => {
            commands::bench_missing::run(&repo_dir, &name, force, dry_run)?;
        }
    }

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_target(false)
        .try_init();
}

fn assert_submodule_initialized(submodule_dir: &Path) -> Result<()> {
    let git_file = submodule_dir.join(".git");
    info!("checking if {git_file:?} exists");
    if !git_file.exists() {
        bail!(
            "子模块 {} 未初始化，请运行 `git submodule update --init --recursive`",
            submodule_dir.display()
        );
    }
    Ok(())
}

fn ensure_clean(repo_dir: &Path) -> Result<()> {
    let status = run_git(repo_dir, ["status", "--porcelain"])?;
    if status.trim().is_empty() {
        Ok(())
    } else {
        bail!(
            "子模块工作区存在未提交的修改，请先清理或使用 --allow-dirty 跳过：\n{}",
            status
        );
    }
}
