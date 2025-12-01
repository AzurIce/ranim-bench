import { CommitRecord } from '../types';
import { COLORS } from '../constants';

interface SidebarProps {
  machines: string[];
  selectedMachines: string[];
  onToggleMachine: (machine: string) => void;
  selectedCommit: string | null;
  commits: CommitRecord[];
  benchmarkCount: number;
  onClearSelection: () => void;
}

export function Sidebar({ 
  machines, 
  selectedMachines, 
  onToggleMachine, 
  selectedCommit, 
  commits, 
  benchmarkCount,
  onClearSelection 
}: SidebarProps) {
  return (
    <aside className="w-full lg:w-64 shrink-0 space-y-6">
      <div className="bg-white p-5 rounded-xl shadow-sm border border-gray-200">
        <h3 className="text-sm font-bold text-gray-400 uppercase tracking-wider mb-4">Machines</h3>
        <div className="space-y-2">
          {machines.map((m, idx) => {
            const isSelected = selectedMachines.includes(m);
            const color = COLORS[idx % COLORS.length];
            return (
              <label key={m} className="flex items-center gap-3 cursor-pointer hover:bg-gray-50 p-2 rounded-lg -mx-2 transition-colors">
                <input
                  type="checkbox"
                  className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                  checked={isSelected}
                  onChange={() => onToggleMachine(m)}
                />
                <span className="flex-1 font-medium text-gray-700">{m}</span>
                {isSelected && <div className="w-3 h-3 rounded-full" style={{ backgroundColor: color }} />}
              </label>
            );
          })}
        </div>
      </div>

      {selectedCommit && (
        <div className="bg-blue-50 p-5 rounded-xl shadow-sm border border-blue-200">
          <h3 className="text-sm font-bold text-blue-800 uppercase tracking-wider mb-2">Selected Commit</h3>
          <div className="text-sm text-blue-900">
            <p className="font-mono text-xs mb-1">{selectedCommit.substring(0, 7)}</p>
            {(() => {
              const c = commits.find(c => c.hash === selectedCommit);
              if (!c) return null;
              return (
                <>
                  {c.refs && (
                    <p className="text-xs font-semibold text-blue-700 mb-1 bg-blue-100 px-1.5 py-0.5 rounded inline-block break-all mr-1">
                      {c.refs}
                    </p>
                  )}
                  {c.branches && c.branches.length > 0 && (
                    <p className="text-xs font-semibold text-green-700 mb-1 bg-green-100 px-1.5 py-0.5 rounded inline-block break-all">
                      {c.branches.join(', ')}
                    </p>
                  )}
                  <p>{c.message}</p>
                  <p className="text-xs text-gray-500 mt-2">{c.date.split('T')[0]} by {c.author}</p>
                </>
              );
            })()}
          </div>
          <button
            className="mt-3 text-xs text-blue-600 hover:underline"
            onClick={onClearSelection}
          >
            Clear selection
          </button>
        </div>
      )}

      <div className="bg-white p-5 rounded-xl shadow-sm border border-gray-200">
        <h3 className="text-sm font-bold text-gray-400 uppercase tracking-wider mb-4">Details</h3>
        <p className="text-sm text-gray-600">
          <span className="block mb-1">Commits: {commits.length}</span>
          <span className="block">Benchmarks: {benchmarkCount}</span>
        </p>
      </div>
    </aside>
  );
}
