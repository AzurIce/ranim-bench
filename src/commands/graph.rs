use crate::common::{CommitRecord, DbManifest};
use crate::utils::{run_git, save_json};
use anyhow::{Result, anyhow};
use git_graph::graph::GitGraph;
use git_graph::print::format::CommitFormat;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, Settings,
};
use git2::Repository;
use std::path::Path;
use tracing::info;

pub fn run(root_dir: &Path, repo_dir: &Path) -> Result<()> {
    // 1. Load manifest
    info!("Loading manifest...");
    let db_root = root_dir.join("db");
    // We load it just to ensure it exists or init it, though we don't strictly need it for the graph
    // if we are showing all history.
    let _manifest = DbManifest::load_or_init(&db_root)?;

    info!("fetching repo");
    run_git(repo_dir, &["fetch", "--all"])?;

    // 2. Open repo
    info!("Opening repository at {}...", repo_dir.display());
    let repo = Repository::open(repo_dir)?;

    // 2.5. Fetch and sync remotes
    // const REMOTE: &str = "origin";
    // info!("Syncing local branches with remote branches...");
    // CLI git fetch already fetched remotes.
    // sync_local_branches_with_remote(&repo, REMOTE)?;

    // 3. Setup GitGraph settings
    let branch_settings = BranchSettings::from(BranchSettingsDef::simple())
        .map_err(|e| anyhow!("Failed to create branch settings: {}", e))?;

    let settings = Settings {
        reverse_commit_order: false,
        debug: false,
        compact: false,
        colored: true,
        include_remote: true,
        format: CommitFormat::Medium, // Not used for internal record building but required
        wrapping: None,
        characters: Characters::thin(),
        branch_order: BranchOrder::ShortestFirst(true),
        branches: branch_settings,
        merge_patterns: MergePatterns::default(),
    };

    // 4. Generate Graph
    info!("Generating git graph...");
    // GitGraph consumes the repo, so we open it again or move it?
    // GitGraph::new takes `repository: Repository`.
    let graph =
        GitGraph::new(repo, &settings, None, None).map_err(|e| anyhow!("GitGraph error: {}", e))?;

    // 5. Build CommitRecords
    info!(
        "Building commit records for {} commits...",
        graph.commits.len()
    );
    let mut records = Vec::new();

    // GitGraph commits are sorted by topological/time usually (depends on implementation).
    // `graph.commits` is a Vec<CommitInfo>.
    // We iterate them to build records.

    // We need to look up parents' hashes from `graph.commits` or `graph.repository`?
    // `CommitInfo` stores `oid`.

    for (_i, commit_info) in graph.commits.iter().enumerate() {
        let commit = graph.commit(commit_info.oid)?;

        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();

        let time = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
            .unwrap_or_default()
            .to_rfc3339();

        let message = commit.message().unwrap_or("").trim().to_string();

        // Refs: git-graph doesn't store refs string on CommitInfo directly in a simple way for us,
        // but it has `tags` and `branches` indices.
        // We can reconstruct a ref string or just use what we have.
        // Let's try to construct a helpful string.
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

        // Branch trace info for layout
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
            branches: branch_names, // Main branch for this commit
            column,
            color,
        });
    }

    // 6. Save
    let web_public_dir = root_dir.join("web").join("public");
    let graph_path = web_public_dir.join("git-graph.json");

    save_json(&graph_path, &records)?;
    info!(
        "Saved {} commits to {}",
        records.len(),
        graph_path.display()
    );

    // Copy db
    let public_db_dir = web_public_dir.join("db");
    copy_dir_recursive(&db_root, &public_db_dir)?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let dst_path = dst.join(&name);
        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            std::fs::copy(&path, &dst_path)?;
        }
    }
    Ok(())
}
