use crate::common::{CpuInfo, SystemInfo};
use anyhow::{anyhow, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_json<T: Serialize>(path: impl AsRef<Path>, data: &T) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, serde_json::to_string_pretty(data)?)?;
    Ok(())
}

pub fn run_git(repo_dir: &Path, args: impl IntoIterator<Item = impl AsRef<str>>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir);
    for arg in args {
        cmd.arg(arg.as_ref());
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("执行 git 命令? বিধ{}", repo_dir.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "git 命令执行失败：{}
{}",
            output.status,
            stderr
        ));
    }
    Ok(String::from_utf8(output.stdout).context("解析 git 输出?UTF-8")?)
}

pub fn collect_system_info() -> SystemInfo {
    use sysinfo::System;
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

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("failed to request wgpu adapter");
    let wgpu_adapter_info = adapter.get_info();

    SystemInfo {
        kernel_version: Some(System::kernel_long_version()),
        os_version: System::long_os_version().unwrap_or_default(),
        distribution_id: System::distribution_id(),
        arch: System::cpu_arch(),
        memory: sys.total_memory(),
        cpus,
        wgpu_adapter_info,
    }
}
