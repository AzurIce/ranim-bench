use crate::common::{BenchmarkEvent, DbManifest, RunManifest};
use crate::utils::{collect_system_info, run_git, save_json};
use anyhow::{Context, Result, anyhow};
use std::io::{BufRead, BufReader};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tracing::{error, info, warn};

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
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

pub fn run(repo_dir: &Path, name: &str, force: bool) -> Result<()> {
    let benches_dir = repo_dir.join("benches");
    let commit_hash = run_git(repo_dir, ["rev-parse", "HEAD"])?.trim().to_string();

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_root = root_dir.join("db");
    let run_dir = db_root.join(&commit_hash).join(name);

    info!("benchmark output will be saved to {}", run_dir.display());
    if run_dir.exists() {
        if !force {
            error!("output directory already exists, use --force to overwrite");
            return Ok(());
        }
        warn!("output directory already exists, removing because --force is specified");
        std::fs::remove_dir_all(&run_dir).context("failed to remove output directory")?;
    }

    // Ensure parent dir exists
    std::fs::create_dir_all(&run_dir)?;

    let system_info = collect_system_info();

    info!("running criterion benchmark...");
    let child = Command::new("cargo")
        .current_dir(&benches_dir)
        .arg("criterion")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn cargo criterion")?;
    let mut child = ChildGuard(child);

    let stdout = child.stdout.take().unwrap();
    let mut stdout = BufReader::new(stdout);

    let mut buf = String::new();
    let mut bench_ids = Vec::new();

    while let Ok(len) = stdout.read_line(&mut buf) {
        if len == 0 {
            break;
        }
        // Parse event
        if let Ok(event) = serde_json::from_str::<BenchmarkEvent>(&buf) {
            match event {
                BenchmarkEvent::BenchmarkComplete(evt) => {
                    info!("benchmark `{}` complete.", evt.id);
                    save_json(run_dir.join(&evt.id).with_extension("json"), &evt.data)?;
                    bench_ids.push(evt.id);
                }
                BenchmarkEvent::GroupComplete(evt) => {
                    info!(
                        "benchmark group `{} {:?}` complete.",
                        evt.group_name, evt.benchmarks
                    );
                    save_json(
                        run_dir
                            .join(&evt.group_name)
                            .join("group")
                            .with_extension("json"),
                        &evt,
                    )?;
                }
            }
        }
        buf.clear();
    }

    let res = child.wait()?;
    if !res.success() {
        return Err(anyhow!("cargo bench failed with code {:?}", res.code()));
    }

    // Save RunManifest
    let run_manifest = RunManifest {
        commit_hash: commit_hash.clone(),
        name: name.to_string(),
        system: system_info,
        benchmarks: bench_ids,
    };
    run_manifest.save(&db_root)?; // Note: RunManifest::save takes db_root and constructs path using hash/name.

    // Update global manifest
    update_db_manifest(&db_root, &commit_hash, name)?;

    Ok(())
}

fn update_db_manifest(db_root: &Path, hash: &str, run_name: &str) -> Result<()> {
    let mut manifest = DbManifest::load_or_init(db_root)?;

    let runs = manifest.benches.entry(hash.to_string()).or_default();
    if !runs.contains(&run_name.to_string()) {
        runs.push(run_name.to_string());
    }

    manifest.save(db_root)?;
    Ok(())
}
