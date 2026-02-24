use crate::common::{RunManifest, SystemInfo};
use crate::utils::save_json;
use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

pub fn run(root_dir: &Path) -> Result<()> {
    let db_dir = root_dir.join("db");
    if !db_dir.exists() {
        warn!("db directory not found at {}", db_dir.display());
        return Ok(());
    }

    info!("Syncing run.json files...");
    let mut count = 0;

    for entry in std::fs::read_dir(&db_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let commit_hash = entry.file_name().into_string().unwrap();

            if commit_hash.len() != 40 {
                continue;
            }

            for run_entry in std::fs::read_dir(&path)? {
                let run_entry = run_entry?;
                let run_path = run_entry.path();
                if run_path.is_dir() {
                    let run_name = run_entry.file_name().into_string().unwrap();

                    if let Err(e) = ensure_run_json(&run_path, &commit_hash, &run_name) {
                        warn!(
                            "Failed to sync run.json for {}/{}: {}",
                            commit_hash, run_name, e
                        );
                    } else {
                        count += 1;
                    }
                }
            }
        }
    }

    info!("Synced {} run.json files.", count);
    Ok(())
}

fn ensure_run_json(run_dir: &Path, commit_hash: &str, run_name: &str) -> Result<()> {
    let run_json_path = run_dir.join("run.json");

    // Try to find system info
    let mut system_info: Option<SystemInfo> = None;

    if run_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&run_json_path) {
            if let Ok(run_manifest) = serde_json::from_str::<RunManifest>(&content) {
                system_info = Some(run_manifest.system);
            }
        }
    }

    // Fallback: check old system_info.json
    let old_sys_info_path = run_dir.join("system_info.json");
    if system_info.is_none() && old_sys_info_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&old_sys_info_path) {
            if let Ok(run_manifest) = serde_json::from_str::<RunManifest>(&content) {
                system_info = Some(run_manifest.system);
            } else if let Ok(sys) = serde_json::from_str::<SystemInfo>(&content) {
                system_info = Some(sys);
            }
        }
    }

    if system_info.is_none() {
        return Err(anyhow::anyhow!("Missing system_info.json or run.json"));
    }

    let system_info = system_info.unwrap();

    // Scan for benchmarks
    let mut benchmarks = Vec::new();
    for entry in std::fs::read_dir(run_dir)? {
        let entry = entry?;
        let name = entry.file_name().into_string().unwrap();

        if name == "run.json" || name == "system_info.json" {
            continue;
        }
        if name.ends_with(".json") {
            let id = name.strip_suffix(".json").unwrap().to_string();
            benchmarks.push(id);
        } else if entry.path().is_dir() {
            collect_bench_ids(&entry.path(), &name, &mut benchmarks)?;
        }
    }

    benchmarks.sort();

    let run_manifest = RunManifest {
        commit_hash: commit_hash.to_string(),
        name: run_name.to_string(),
        system: system_info,
        benchmarks,
    };

    save_json(&run_json_path, &run_manifest)?;

    Ok(())
}

fn collect_bench_ids(dir: &Path, prefix: &str, benchmarks: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().into_string().unwrap();
        let path = entry.path();

        if path.is_dir() {
            collect_bench_ids(&path, &format!("{}/{}", prefix, name), benchmarks)?;
        } else if name.ends_with(".json") && name != "group.json" {
            let id_part = name.strip_suffix(".json").unwrap();
            benchmarks.push(format!("{}/{}", prefix, id_part));
        }
    }
    Ok(())
}
