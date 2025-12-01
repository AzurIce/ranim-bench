export interface CommitRecord {
  hash: string;
  parents: string[];
  date: string;
  author: string;
  refs: string;
  message: string;
  branches: string[];
  column?: number;
  color?: string;
}

export interface CpuInfo {
  name: string;
  vendor_id: string;
  brand: string;
  frequency: number;
}

export interface AdapterInfo {
  name: string;
  vendor: number;
  device: number;
  device_type: string;
  driver: string;
  driver_info: string;
  backend: string;
}

export interface SystemInfo {
  kernel_version: string;
  os_version: string;
  distribution_id: string;
  arch: string;
  memory: number;
  cpus: CpuInfo[];
  wgpu_adapter_info: AdapterInfo;
}

export interface RunManifest {
  commit_hash: string;
  name: string;
  system: SystemInfo;
  benchmarks: string[];
}

export interface DbManifest {
  // "hash" -> ["run_name1", "run_name2"]
  benches: Record<string, string[]>;
}

export interface BenchmarkValue {
  estimate: number;
  lower_bound: number;
  upper_bound: number;
  unit: string;
}

export interface BenchmarkResult {
  id: string;
  mean: BenchmarkValue;
}
