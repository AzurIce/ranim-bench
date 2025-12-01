import { SystemInfo } from '../types';

interface SystemInfoViewProps {
  selectedMachines: string[];
  getSystemInfo: (machine: string) => SystemInfo | null;
}

export function SystemInfoView({ selectedMachines, getSystemInfo }: SystemInfoViewProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      {selectedMachines.map(m => {
        const info = getSystemInfo(m);
        if (!info) return null;
        return (
          <div key={m} className="bg-white rounded-xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="bg-gray-50 px-6 py-4 border-b border-gray-200">
              <h3 className="font-bold text-lg">{m}</h3>
            </div>
            <div className="p-6 space-y-4 text-sm">
              <div>
                <h4 className="font-semibold text-gray-500 mb-1">OS</h4>
                <p>{info.os_version} ({info.arch})</p>
                <p className="text-gray-400 text-xs">{info.kernel_version}</p>
              </div>
              <div>
                <h4 className="font-semibold text-gray-500 mb-1">CPU</h4>
                {info.cpus.length > 0 && (
                  <p>{info.cpus[0].brand} ({info.cpus.length} cores)</p>
                )}
              </div>
              <div>
                <h4 className="font-semibold text-gray-500 mb-1">Memory</h4>
                <p>{(info.memory / 1024 / 1024 / 1024).toFixed(1)} GB</p>
              </div>
              <div>
                <h4 className="font-semibold text-gray-500 mb-1">GPU (WGPU)</h4>
                <p>{info.wgpu_adapter_info.name}</p>
                <p className="text-xs text-gray-400">{info.wgpu_adapter_info.backend} - {info.wgpu_adapter_info.driver}</p>
              </div>
            </div>
          </div>
        )
      })}
      {selectedMachines.length === 0 && <div className="text-gray-500 italic">Select a machine to view system info.</div>}
    </div>
  );
}
