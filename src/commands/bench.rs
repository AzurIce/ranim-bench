use crate::common::{BenchmarkEvent, RunManifest};
use crate::utils::{collect_system_info, run_git, save_json};
use anyhow::{anyhow, Context, Result};
use indicatif::ProgressStyle;
use std::io::{BufRead, BufReader};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tracing::{info, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

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
    let tmp_dir = db_root.join(&commit_hash).join(format!("{}.tmp", name));

    info!("benchmark output will be saved to {}", run_dir.display());
    if run_dir.exists() {
        if !force {
            return Err(anyhow!(
                "output directory already exists, use --force to overwrite"
            ));
        }
        warn!("output directory already exists, will overwrite on success");
    }

    // Clean up any leftover tmp dir
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).context("failed to remove leftover tmp directory")?;
    }
    std::fs::create_dir_all(&tmp_dir)?;

    let system_info = collect_system_info();

    info!("running criterion benchmark...");
    let result = run_benchmarks(&benches_dir, &tmp_dir);

    match result {
        Ok(bench_ids) => {
            // Save RunManifest into tmp dir
            let run_manifest = RunManifest {
                commit_hash: commit_hash.clone(),
                name: name.to_string(),
                system: system_info,
                benchmarks: bench_ids,
            };
            save_json(tmp_dir.join("run.json"), &run_manifest)?;

            // Atomically move tmp -> final
            if run_dir.exists() {
                std::fs::remove_dir_all(&run_dir)
                    .context("failed to remove existing output directory")?;
            }
            std::fs::rename(&tmp_dir, &run_dir).context("failed to move tmp dir to final")?;
            info!("benchmark results saved to {}", run_dir.display());
            Ok(())
        }
        Err(e) => {
            // Clean up tmp dir on failure
            warn!("benchmark failed, cleaning up tmp directory");
            let _ = std::fs::remove_dir_all(&tmp_dir);
            Err(e)
        }
    }
}

fn run_benchmarks(benches_dir: &Path, output_dir: &Path) -> Result<Vec<String>> {
    let child = Command::new("cargo")
        .current_dir(benches_dir)
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

    let bench_span = tracing::info_span!("benchmarking");
    bench_span.pb_set_style(
        &ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    let _bench_guard = bench_span.enter();

    while let Ok(len) = stdout.read_line(&mut buf) {
        if len == 0 {
            break;
        }
        if let Ok(event) = serde_json::from_str::<BenchmarkEvent>(&buf) {
            match event {
                BenchmarkEvent::BenchmarkComplete(evt) => {
                    bench_span.pb_inc(1);
                    bench_span.pb_set_message(&evt.id);
                    info!("benchmark `{}` complete.", evt.id);
                    save_json(output_dir.join(&evt.id).with_extension("json"), &evt.data)?;
                    bench_ids.push(evt.id);
                }
                BenchmarkEvent::GroupComplete(evt) => {
                    info!(
                        "benchmark group `{} {:?}` complete.",
                        evt.group_name, evt.benchmarks
                    );
                    save_json(
                        output_dir
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

    Ok(bench_ids)
}
