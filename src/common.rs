use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::info;
use wgpu::AdapterInfo;

use crate::utils::{load_json, save_json};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunManifest {
    pub commit_hash: String,
    pub name: String,
    #[serde(flatten)]
    pub system: SystemInfo,
    /// List of benchmark IDs executed in this run
    pub benchmarks: Vec<String>,
}

impl RunManifest {
    pub fn save(&self, db_root: &Path) -> Result<()> {
        let manifest_path = db_root
            .join(&self.commit_hash)
            .join(&self.name)
            .join("run.json");
        info!("saving run manifest to {}", manifest_path.display());
        save_json(&manifest_path, self)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub kernel_version: Option<String>, // sysinfo return Option or String? check main.rs
    pub os_version: String,
    pub distribution_id: String,
    pub arch: String,
    pub memory: u64,
    pub cpus: Vec<CpuInfo>,
    pub wgpu_adapter_info: AdapterInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CpuInfo {
    pub name: String,
    pub vendor_id: String,
    pub brand: String,
    pub frequency: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DbManifest {
    // Saved as db.json
    pub benches: HashMap<String, Vec<String>>,
}

impl DbManifest {
    pub fn load_or_init(db_root: &Path) -> Result<Self> {
        let manifest_path = db_root.join("db.json");
        if manifest_path.exists() {
            load_json(&manifest_path)
        } else {
            Ok(Self::default())
        }
    }
    pub fn save(&self, db_root: &Path) -> Result<()> {
        let manifest_path = db_root.join("db.json");
        save_json(&manifest_path, self)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CommitRecord {
    pub hash: String,
    pub parents: Vec<String>,
    pub date: String,
    pub author: String,
    pub refs: String,
    pub message: String,
    // User mentioned "branch name". refs usually contains it.
    // Do we need a separate field?
    // "save its branch name (obtained from git-graph)".
    // Maybe the user wants a specific `branch` field.
    // I'll add `branches: Vec<String>` to be safe.
    pub branches: Vec<String>,
    pub column: Option<usize>,
    pub color: Option<String>,
}

// For compatibility with existing logic or if we need to read criterion output
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum BenchmarkEvent {
    BenchmarkComplete(BenchmarkComplete),
    GroupComplete(GroupComplete),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkComplete {
    pub id: String,
    // other fields we might not need to parse fully if we just want IDs,
    // but we strictly parse them in main.rs so let's keep them.
    // To avoid redefining detailed structs, I'll use serde_json::Value for ignored fields if possible,
    // or just minimal fields.
    // actually main.rs had full struct.
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupComplete {
    pub group_name: String,
    pub benchmarks: Vec<String>,
}
