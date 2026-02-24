use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::info;
use wgpu::AdapterInfo;

use crate::utils::save_json;

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
    pub kernel_version: Option<String>,
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

#[derive(Debug, Serialize, Clone)]
pub struct CommitRecord {
    pub hash: String,
    pub parents: Vec<String>,
    pub date: String,
    pub author: String,
    pub refs: String,
    pub message: String,
    pub branches: Vec<String>,
    pub column: Option<usize>,
    pub color: Option<String>,
}

// --- Aggregated data for web frontend ---

#[derive(Debug, Serialize, Default)]
pub struct AllData {
    /// Per-machine system info (from the latest run)
    pub machines: HashMap<String, SystemInfo>,
    /// Per-commit benchmark data
    pub commits: HashMap<String, CommitBenchData>,
}

#[derive(Debug, Serialize, Default)]
pub struct CommitBenchData {
    /// Which machines have data for this commit
    pub machines: Vec<String>,
    /// machine -> bench_id -> BenchValue
    pub benchmarks: HashMap<String, HashMap<String, BenchValue>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchValue {
    pub estimate: f64,
    pub unit: String,
}

// --- Criterion output parsing ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum BenchmarkEvent {
    BenchmarkComplete(BenchmarkComplete),
    GroupComplete(GroupComplete),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkComplete {
    pub id: String,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupComplete {
    pub group_name: String,
    pub benchmarks: Vec<String>,
}
