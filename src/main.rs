use std::fs;
use std::io::{BufRead, BufReader};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{SecondsFormat, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 是否跳过工作区干净性检查
    #[arg(long)]
    allow_dirty: bool,
    /// Save name
    #[arg(long)]
    name: String,
    /// Overwrite existing output directory
    #[arg(long)]
    force: bool,
}

fn main() -> Result<()> {
    init_tracing();

    let repo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ranim");
    assert!(repo_dir.exists());

    let cli = Cli::parse();
    if !cli.allow_dirty {
        ensure_clean(&repo_dir)?;
    } else {
        warn!("allow dirty is true, skipping clean check...");
    }

    let commit_hash = run_git(&repo_dir, ["rev-parse", "HEAD"])?;
    let commit_hash = commit_hash.trim().to_string();
    info!("benchmarking on commit {commit_hash}...");

    info!("running criterion benchmark...");
    run_criterion(&repo_dir, &cli.name, cli.force)?;

    // let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    // let output_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
    //     .join("db")
    //     .join(&commit_hash)
    //     .join(&timestamp);
    // fs::create_dir_all(&output_dir)
    //     .with_context(|| format!("创建输出目录 {}", output_dir.display()))?;

    // let jsonl_path = output_dir.join("cargo-bench.jsonl");
    // fs::write(&jsonl_path, &bench_output.stdout)
    //     .with_context(|| format!("写入 {}", jsonl_path.display()))?;

    // let metadata_path = output_dir.join("metadata.json");
    // let metadata = serde_json::json!({
    //     "commit": commit_hash,
    //     "timestamp": timestamp,
    //     "status": bench_output.status.code(),
    // });
    // fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)
    //     .with_context(|| format!("写入 {}", metadata_path.display()))?;

    // info!("benchmark 输出已保存到 {}", output_dir.display());

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

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkComplete {
    id: String,
    iteration_count: Vec<u64>,
    measured_values: Vec<f64>,
    unit: String,
    typical: ConfidenceInterval,
    mean: ConfidenceInterval,
    median: ConfidenceInterval,
    median_abs_dev: ConfidenceInterval,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfidenceInterval {
    estimate: f64,
    lower_bound: f64,
    upper_bound: f64,
    unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupComplete {
    group_name: String,
    benchmarks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum BenchmarkEvent {
    BenchmarkComplete(BenchmarkComplete),
    GroupComplete(GroupComplete),
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

impl Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn run_criterion(repo_dir: &Path, name: &str, force: bool) -> Result<()> {
    let benches_dir = repo_dir.join("benches");
    let commit_hash = run_git(&repo_dir, ["rev-parse", "HEAD"])?;
    let commit_hash = commit_hash.trim().to_string();
    // let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("db")
        .join(&commit_hash)
        .join(&name);
    info!("benchmark output will be saved to {}", output_dir.display());
    if output_dir.exists() {
        if !force {
            error!("output directory already exists, use --force to overwrite");
            return Ok(());
        }
        warn!("output directory already exists, removing because --force is specified");
        std::fs::remove_dir_all(&output_dir).context("failed to remove output directory")?;
    }

    let system_info = collect_system_info();
    save_json(&output_dir.join("system_info.json"), &system_info).unwrap();
    info!("system info saved to {}", output_dir.join("system_info.json").display());

    let child = Command::new("cargo")
        .current_dir(&benches_dir)
        .arg("criterion")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn cargo criterion")?;
    let mut child= ChildGuard(child);

    let stdout = child.stdout.take().unwrap();
    let mut stdout = BufReader::new(stdout);

    let mut buf = String::new();
    while let Ok(len) = stdout.read_line(&mut buf)
        && len > 0
    {
        // debug!("read line: {len} {buf:?}");
        let event = match serde_json::from_str::<BenchmarkEvent>(&buf) {
            Ok(res) => res,
            Err(err) => {
                warn!("failed to parse benchmark event: {err:?}");
                continue;
            }
        };
        match event {
            BenchmarkEvent::BenchmarkComplete(evt) => {
                info!("benchmark `{}` complete.", evt.id);
                let segments = evt.id.split('/').collect::<Vec<_>>();
                assert!(segments.len() == 2);

                save_json(
                    output_dir
                        .join(segments[0])
                        .join(segments[1])
                        .with_extension("json"),
                    &evt,
                )
                .unwrap()
            }
            BenchmarkEvent::GroupComplete(evt) => {
                info!(
                    "benchmark group `{} {:?}` complete.",
                    evt.group_name, evt.benchmarks
                );
                save_json(
                    &output_dir
                        .join(&evt.group_name)
                        .join("group")
                        .with_extension("json"),
                    &evt,
                )
                .unwrap()
            }
        }
        buf.clear();
    }

    let res = child.wait()?;
    if !res.success() {
        return Err(anyhow!("cargo bench failed with code {:?}", res.code()));
    }

    Ok(())
}

fn save_json<T: Serialize>(path: impl AsRef<Path>, data: &T) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, serde_json::to_string_pretty(data).unwrap()).unwrap();
    Ok(())
}

fn run_git(repo_dir: &Path, args: impl IntoIterator<Item = impl AsRef<str>>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir);
    for arg in args {
        cmd.arg(arg.as_ref());
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("执行 git 命令于 {}", repo_dir.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git 命令执行失败：{}\n{}", output.status, stderr));
    }
    Ok(String::from_utf8(output.stdout).with_context(|| "解析 git 输出为 UTF-8")?)
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemInfo {
    kernel_version: String,
    os_version: String,
    distribution_id: String,
    arch: String,
    /// Bytes
    memory: u64,
    cpus: Vec<CpuInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CpuInfo {
    name: String,
    vendor_id: String,
    brand: String,
    frequency: u64,
}

fn collect_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpus = sys
        .cpus()
        .iter()
        .map(|cpu| CpuInfo {
            name: cpu.name().to_string(),
            vendor_id: cpu.vendor_id().to_string(),
            brand: cpu.brand().to_string(),
            frequency: cpu.frequency(),
        })
        .collect();

    SystemInfo {
        kernel_version: System::kernel_long_version(),
        os_version: System::long_os_version().unwrap_or_default(),
        distribution_id: System::distribution_id(),
        arch: System::cpu_arch(),
        memory: sys.total_memory(),
        cpus,
    }
}

#[test]
fn foo() {
    dbg!(collect_system_info());
}
