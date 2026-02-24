use crate::common::{AllData, BenchValue, CommitBenchData, CommitRecord, RunManifest};
use crate::utils::{load_json, run_git, save_json};
use anyhow::{anyhow, Result};
use git2::Repository;
use git_graph::graph::GitGraph;
use git_graph::print::format::CommitFormat;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, Settings,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

pub fn run(root_dir: &Path, repo_dir: &Path) -> Result<()> {
    let db_root = root_dir.join("db");
    let web_public_dir = root_dir.join("web").join("public");

    // 1. Scan db/ and build aggregated data
    info!("Scanning db/ for benchmark data...");
    let all_data = scan_db(&db_root)?;
    info!(
        "Found {} commits, {} machines",
        all_data.commits.len(),
        all_data.machines.len()
    );

    // 2. Generate git-graph
    info!("Fetching repo...");
    run_git(repo_dir, &["fetch", "--all"])?;

    info!("Opening repository at {}...", repo_dir.display());
    let repo = Repository::open(repo_dir)?;

    let branch_settings = BranchSettings::from(BranchSettingsDef::simple())
        .map_err(|e| anyhow!("Failed to create branch settings: {}", e))?;

    let settings = Settings {
        reverse_commit_order: false,
        debug: false,
        compact: false,
        colored: true,
        include_remote: true,
        format: CommitFormat::Medium,
        wrapping: None,
        characters: Characters::thin(),
        branch_order: BranchOrder::ShortestFirst(true),
        branches: branch_settings,
        merge_patterns: MergePatterns::default(),
    };

    info!("Generating git graph...");
    let graph =
        GitGraph::new(repo, &settings, None, None).map_err(|e| anyhow!("GitGraph error: {}", e))?;

    info!(
        "Building commit records for {} commits...",
        graph.commits.len()
    );
    let mut records = Vec::new();

    for (_i, commit_info) in graph.commits.iter().enumerate() {
        let commit = graph.commit(commit_info.oid)?;
        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let time = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
            .unwrap_or_default()
            .to_rfc3339();
        let message = commit.message().unwrap_or("").trim().to_string();

        let mut refs_parts = Vec::new();
        for &branch_idx in &commit_info.branches {
            if let Some(branch) = graph.all_branches.get(branch_idx) {
                refs_parts.push(branch.name.clone());
            }
        }
        for &tag_idx in &commit_info.tags {
            if let Some(tag) = graph.all_branches.get(tag_idx) {
                refs_parts.push(format!("tag: {}", tag.name));
            }
        }
        let refs_list = refs_parts.join(", ");

        let (column, color, branch_names) = if let Some(trace_idx) = commit_info.branch_trace {
            if let Some(branch) = graph.all_branches.get(trace_idx) {
                (
                    branch.visual.column,
                    Some(branch.visual.svg_color.clone()),
                    vec![branch.name.clone()],
                )
            } else {
                (None, None, vec![])
            }
        } else {
            (None, None, vec![])
        };

        records.push(CommitRecord {
            hash: commit_info.oid.to_string(),
            parents,
            date: time,
            author: author_name,
            refs: refs_list,
            message,
            branches: branch_names,
            column,
            color,
        });
    }

    // 3. Save outputs
    save_json(web_public_dir.join("git-graph.json"), &records)?;
    info!("Saved {} commits to git-graph.json", records.len());

    save_json(web_public_dir.join("all-data.json"), &all_data)?;
    info!("Saved all-data.json");

    Ok(())
}

/// Scan the db/ directory and build aggregated AllData
fn scan_db(db_root: &Path) -> Result<AllData> {
    let mut all_data = AllData::default();

    if !db_root.exists() {
        return Ok(all_data);
    }

    for entry in std::fs::read_dir(db_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let commit_hash = entry.file_name().into_string().unwrap();
        // Only process 40-char hex directories (commit hashes)
        if commit_hash.len() != 40 {
            continue;
        }

        let mut commit_data = CommitBenchData::default();

        for run_entry in std::fs::read_dir(&path)? {
            let run_entry = run_entry?;
            let run_path = run_entry.path();
            if !run_path.is_dir() {
                continue;
            }

            let machine_name = run_entry.file_name().into_string().unwrap();

            // Load run.json for system info
            let run_json_path = run_path.join("run.json");
            if !run_json_path.exists() {
                warn!("Missing run.json for {}/{}", commit_hash, machine_name);
                continue;
            }

            let run_manifest: RunManifest = match load_json(&run_json_path) {
                Ok(m) => m,
                Err(e) => {
                    warn!(
                        "Failed to parse run.json for {}/{}: {}",
                        commit_hash, machine_name, e
                    );
                    continue;
                }
            };

            // Update machine system info (keep the latest one seen)
            all_data
                .machines
                .insert(machine_name.clone(), run_manifest.system);

            // Load each benchmark result
            let mut bench_results = HashMap::new();
            for bench_id in &run_manifest.benchmarks {
                let bench_path = run_path.join(bench_id).with_extension("json");
                if let Ok(json) = std::fs::read_to_string(&bench_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                        if let (Some(estimate), Some(unit)) = (
                            val.get("mean")
                                .and_then(|m| m.get("estimate"))
                                .and_then(|e| e.as_f64()),
                            val.get("mean")
                                .and_then(|m| m.get("unit"))
                                .and_then(|u| u.as_str()),
                        ) {
                            bench_results.insert(
                                bench_id.clone(),
                                BenchValue {
                                    estimate,
                                    unit: unit.to_string(),
                                },
                            );
                        }
                    }
                }
            }

            commit_data.machines.push(machine_name.clone());
            commit_data.benchmarks.insert(machine_name, bench_results);
        }

        if !commit_data.machines.is_empty() {
            all_data.commits.insert(commit_hash, commit_data);
        }
    }

    Ok(all_data)
}
