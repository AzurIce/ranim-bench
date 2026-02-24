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

export interface BenchValue {
  estimate: number;
  unit: string;
}

export interface CommitBenchData {
  machines: string[];
  benchmarks: Record<string, Record<string, BenchValue>>;
}

export interface AllData {
  machines: Record<string, SystemInfo>;
  commits: Record<string, CommitBenchData>;
}
