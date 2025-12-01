import { useEffect, useRef, useState } from 'react';
import { CommitGraph } from './components/CommitGraph';
import { Navbar } from './components/Navbar';
import { Sidebar } from './components/Sidebar';
import { SystemInfoView } from './components/SystemInfoView';
import { BenchmarkCharts } from './components/BenchmarkCharts';
import { useAppData } from './hooks/useAppData';

function App() {
  const {
    commits,
    loading,
    selectedCommit,
    setSelectedCommit,
    selectedMachines,
    toggleMachine,
    benchmarkData,
    machines,
    chartGroups,
    commitsWithData,
    getSystemInfo
  } = useAppData();

  const [activeTab, setActiveTab] = useState<'charts' | 'system'>('charts');
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Scroll to end of graph on load
  useEffect(() => {
    if (scrollContainerRef.current && commits.length > 0) {
      scrollContainerRef.current.scrollLeft = scrollContainerRef.current.scrollWidth;
    }
  }, [commits]);

  if (loading) {
    return <div className="flex items-center justify-center h-screen text-gray-500">Loading benchmark data...</div>;
  }

  return (
    <div className="min-h-screen bg-gray-50 text-gray-900 font-sans">
      <Navbar activeTab={activeTab} onTabChange={setActiveTab} />

      {/* Git Graph Area */}
      <div ref={scrollContainerRef} className="bg-gray-100 border-b border-gray-300 overflow-x-auto">
        <div className="min-w-max p-4">
          {commits.length > 0 ? (
            <CommitGraph
              commits={commits}
              selectedCommit={selectedCommit}
              onCommitSelect={setSelectedCommit}
              commitsWithData={commitsWithData}
            />
          ) : (
            <div className="text-gray-400 text-sm">Loading graph...</div>
          )}
        </div>
      </div>

      <div className="container mx-auto p-6 flex flex-col lg:flex-row gap-8">
        <Sidebar
          machines={machines}
          selectedMachines={selectedMachines}
          onToggleMachine={toggleMachine}
          selectedCommit={selectedCommit}
          commits={commits}
          benchmarkCount={Object.keys(benchmarkData).length}
          onClearSelection={() => setSelectedCommit(null)}
        />

        <main className="flex-1 min-w-0">
          {activeTab === 'system' && (
            <SystemInfoView 
              selectedMachines={selectedMachines}
              getSystemInfo={getSystemInfo}
            />
          )}

          {activeTab === 'charts' && (
            <BenchmarkCharts 
              chartGroups={chartGroups}
              benchmarkData={benchmarkData}
              selectedCommit={selectedCommit}
              selectedMachines={selectedMachines}
              machines={machines}
            />
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
