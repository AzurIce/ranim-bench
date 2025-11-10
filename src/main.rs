use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::Serialize;
use serde_json::Value;
use sysinfo::System;

#[derive(Parser, Debug)]
#[command(author, version, about = "运行 ranim 基准测试并保存结果", long_about = None)]
struct Cli {
    /// ranim 仓库内需要基准测试的 commit（可为 hash、分支或 tag）
    #[arg(long)]
    commit: String,

    /// 指定要运行的基准测试（--bench 名称），默认运行所有
    #[arg(long = "bench", value_name = "BENCH")]
    benches: Vec<String>,

    /// 指定输出目录，默认写入项目根目录下 db/{timestamp}
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ranim_repo = workspace_root.join("ranim");
    if !ranim_repo.exists() {
        bail!("未找到 ranim 子模块目录：{}", ranim_repo.display());
    }

    let resolved_commit = resolve_commit(&ranim_repo, cli.commit.trim())?;

    let start_time = Utc::now();
    let timestamp_str = start_time.format("%Y%m%dT%H%M%SZ").to_string();

    let output_dir = cli
        .output
        .unwrap_or_else(|| workspace_root.join("db").join(&timestamp_str));
    if output_dir.exists() {
        bail!("输出目录已存在：{}", output_dir.display());
    }
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("创建输出目录失败：{}", output_dir.display()))?;

    let worktree_label = format!("bench-{}", timestamp_str);
    let worktree = WorktreeGuard::create(&ranim_repo, &resolved_commit, &worktree_label)
        .context("创建临时 worktree 失败")?;

    let bench_crate = worktree.path().join("benches");
    if !bench_crate.exists() {
        bail!(
            "在临时 worktree 中未找到 benches 包：{}",
            bench_crate.display()
        );
    }

    let benches = cli.benches.clone();
    let bench_result =
        run_benchmarks(&bench_crate, &benches).context("运行 criterion 基准测试失败")?;

    let json_path = output_dir.join("criterion.json");
    let stdout_path = output_dir.join("stdout.log");
    let stderr_path = output_dir.join("stderr.log");
    store_json(&json_path, &bench_result.records).context("写入 criterion.json 失败")?;
    fs::write(&stdout_path, &bench_result.stdout)
        .with_context(|| format!("写入 stdout.log 失败：{}", stdout_path.display()))?;
    fs::write(&stderr_path, &bench_result.stderr)
        .with_context(|| format!("写入 stderr.log 失败：{}", stderr_path.display()))?;

    let metadata = Metadata::gather(
        start_time,
        &resolved_commit,
        &benches,
        &bench_result,
        &workspace_root,
        &ranim_repo,
    )?;

    let metadata_path = output_dir.join("metadata.toml");
    let metadata_toml = toml::to_string_pretty(&metadata).context("生成 metadata TOML 失败")?;
    fs::write(&metadata_path, metadata_toml)
        .with_context(|| format!("写入 metadata.toml 失败：{}", metadata_path.display()))?;

    if bench_result.records.is_empty() {
        eprintln!("警告：未从基准测试输出中解析到任何 JSON 记录。");
    }

    println!(
        "基准测试完成。\n  JSON 结果：{}\n  STDOUT：{}\n  STDERR：{}\n  元信息：{}",
        json_path.display(),
        stdout_path.display(),
        stderr_path.display(),
        metadata_path.display()
    );

    Ok(())
}

struct WorktreeGuard {
    repo: PathBuf,
    path: PathBuf,
}

impl WorktreeGuard {
    fn create(repo: &Path, commit: &str, label: &str) -> Result<Self> {
        let worktree_root = repo.join(".bench_worktrees");
        fs::create_dir_all(&worktree_root)
            .with_context(|| format!("创建 worktree 根目录失败：{}", worktree_root.display()))?;

        let mut worktree_path = worktree_root.join(label);
        let mut counter = 0;
        while worktree_path.exists() {
            counter += 1;
            worktree_path = worktree_root.join(format!("{label}-{counter}"));
        }

        let worktree_str = worktree_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("worktree 路径包含非 UTF-8 字符"))?
            .to_string();

        git(
            repo,
            [
                "worktree",
                "add",
                "--force",
                "--detach",
                worktree_str.as_str(),
                commit,
            ],
        )?;

        Ok(Self {
            repo: repo.to_path_buf(),
            path: worktree_path,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let worktree_str = match self.path.to_str() {
            Some(s) => s.to_string(),
            None => {
                eprintln!("警告：worktree 路径包含非 UTF-8 字符，无法自动清理");
                return;
            }
        };

        if let Err(err) = git(
            &self.repo,
            ["worktree", "remove", "--force", worktree_str.as_str()],
        ) {
            eprintln!(
                "警告：移除临时 worktree 失败（{}）：{err:?}",
                self.path.display()
            );
        }
    }
}

struct BenchmarkResult {
    records: Vec<Value>,
    command: Vec<String>,
    duration: Duration,
    stdout: String,
    stderr: String,
}

fn run_benchmarks(bench_crate: &Path, benches: &[String]) -> Result<BenchmarkResult> {
    let mut command = vec!["cargo".to_string(), "bench".to_string()];
    for bench in benches {
        command.push("--bench".into());
        command.push(bench.clone());
    }
    command.push("--".into());
    command.push("--message-format=json".into());

    let mut cmd = Command::new("cargo");
    cmd.arg("bench");
    for bench in benches {
        cmd.arg("--bench").arg(bench);
    }
    cmd.arg("--");
    cmd.arg("--message-format=json");
    cmd.current_dir(bench_crate);

    let started = Instant::now();
    let output = cmd.output().context("执行 cargo bench 失败")?;
    let duration = started.elapsed();

    let stdout = String::from_utf8(output.stdout).context("基准测试输出不是 UTF-8")?;
    let stderr = String::from_utf8(output.stderr).context("基准测试错误输出不是 UTF-8")?;

    if !output.status.success() {
        bail!(
            "cargo bench 执行失败（退出码 {:?}）：\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    let mut records = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => records.push(value),
            Err(_) => {
                // 非 JSON 行，忽略
            }
        }
    }

    Ok(BenchmarkResult {
        records,
        command,
        duration,
        stdout,
        stderr,
    })
}

fn store_json(path: &Path, records: &[Value]) -> Result<()> {
    let json = serde_json::to_string_pretty(records).context("序列化 JSON 失败")?;
    fs::write(path, json).with_context(|| format!("写入 JSON 文件失败：{}", path.display()))?;
    Ok(())
}

fn resolve_commit(repo: &Path, commit: &str) -> Result<String> {
    if commit.is_empty() {
        bail!("commit 不能为空");
    }
    let output = git(repo, ["rev-parse", "--verify", commit])?;
    if output.is_empty() {
        bail!("无法解析 commit：{commit}");
    }
    Ok(output)
}

#[derive(Serialize)]
struct Metadata {
    timestamp: String,
    commit: String,
    commit_short: String,
    benches: Vec<String>,
    duration_seconds: f64,
    record_count: usize,
    command: Vec<String>,
    system: SystemMetadata,
    git: GitMetadata,
    toolchain: ToolchainMetadata,
}

impl Metadata {
    fn gather(
        timestamp: DateTime<Utc>,
        commit: &str,
        benches: &[String],
        bench_result: &BenchmarkResult,
        workspace_root: &Path,
        repo: &Path,
    ) -> Result<Self> {
        let commit_short = git(repo, ["rev-parse", "--short", commit])
            .unwrap_or_else(|_| commit.chars().take(7).collect());

        let system = SystemMetadata::capture();
        let git = GitMetadata::capture(workspace_root, repo);
        let toolchain = ToolchainMetadata::capture();

        Ok(Self {
            timestamp: timestamp.to_rfc3339(),
            commit: commit.to_string(),
            commit_short,
            benches: benches.to_vec(),
            duration_seconds: bench_result.duration.as_secs_f64(),
            record_count: bench_result.records.len(),
            command: bench_result.command.clone(),
            system,
            git,
            toolchain,
        })
    }
}

#[derive(Serialize)]
struct SystemMetadata {
    host_name: Option<String>,
    os_name: Option<String>,
    os_version: Option<String>,
    kernel_version: Option<String>,
    distribution: Option<String>,
    architecture: String,
    cpu_brand: String,
    cpu_frequency_mhz: Option<u64>,
    cpu_physical_cores: Option<usize>,
    cpu_logical_cores: usize,
    total_memory_bytes: u64,
    used_memory_bytes: u64,
    total_swap_bytes: u64,
    used_swap_bytes: u64,
    gpu: Option<String>,
}

impl SystemMetadata {
    fn capture() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let cpu = system.global_cpu_info();
        let cpu_brand = cpu.brand().trim().to_owned();
        let cpu_frequency_mhz = cpu.frequency();

        let total_memory_bytes = system.total_memory() * 1024;
        let used_memory_bytes = system.used_memory() * 1024;
        let total_swap_bytes = system.total_swap() * 1024;
        let used_swap_bytes = system.used_swap() * 1024;

        SystemMetadata {
            host_name: System::host_name(),
            os_name: System::name(),
            os_version: System::long_os_version(),
            kernel_version: System::kernel_version(),
            distribution: {
                let distribution = System::distribution_id();
                if distribution.is_empty() {
                    None
                } else {
                    Some(distribution)
                }
            },
            architecture: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            cpu_brand,
            cpu_frequency_mhz: if cpu_frequency_mhz == 0 {
                None
            } else {
                Some(cpu_frequency_mhz.into())
            },
            cpu_physical_cores: system.physical_core_count(),
            cpu_logical_cores: system.cpus().len(),
            total_memory_bytes,
            used_memory_bytes,
            total_swap_bytes,
            used_swap_bytes,
            gpu: None,
        }
    }
}

#[derive(Serialize)]
struct GitMetadata {
    workspace_commit: Option<String>,
    workspace_commit_short: Option<String>,
    ranim_remote: Option<String>,
}

impl GitMetadata {
    fn capture(workspace_root: &Path, ranim_repo: &Path) -> Self {
        let workspace_commit = git(workspace_root, ["rev-parse", "HEAD"])
            .ok()
            .filter(|s| !s.is_empty());
        let workspace_commit_short = workspace_commit
            .as_ref()
            .and_then(|commit| git(workspace_root, ["rev-parse", "--short", commit]).ok());

        let ranim_remote = git(ranim_repo, ["remote", "get-url", "origin"])
            .ok()
            .filter(|s| !s.is_empty());

        GitMetadata {
            workspace_commit,
            workspace_commit_short,
            ranim_remote,
        }
    }
}

#[derive(Serialize)]
struct ToolchainMetadata {
    rustc: Option<String>,
    cargo: Option<String>,
}

impl ToolchainMetadata {
    fn capture() -> Self {
        ToolchainMetadata {
            rustc: command_version("rustc"),
            cargo: command_version("cargo"),
        }
    }
}

fn command_version(cmd: &str) -> Option<String> {
    let output = Command::new(cmd).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_string())
}

fn git(repo: &Path, args: impl IntoIterator<Item = impl AsRef<str>>) -> Result<String> {
    let args_vec: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();

    let mut command = Command::new("git");
    command.arg("-C");
    command.arg(repo);
    for arg in &args_vec {
        command.arg(arg);
    }

    let output = command
        .output()
        .with_context(|| format!("执行 git {:?} 失败", args_vec))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {:?} 失败：{}", args_vec, stderr.trim());
    }

    let stdout = String::from_utf8(output.stdout).context("git 输出不是 UTF-8")?;
    Ok(stdout.trim().to_string())
}
