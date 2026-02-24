import { useState, useEffect, useMemo } from 'react';
import { groupBy } from 'lodash';
import { CommitRecord, AllData, SystemInfo } from '../types';

export function useAppData() {
  const [commits, setCommits] = useState<CommitRecord[]>([]);
  const [allData, setAllData] = useState<AllData | null>(null);
  const [selectedMachines, setSelectedMachines] = useState<string[]>([]);
  const [benchmarkData, setBenchmarkData] = useState<Record<string, any[]>>({});
  const [loading, setLoading] = useState(true);
  const [selectedCommit, setSelectedCommit] = useState<string | null>(null);

  // Load all data in two fetches
  useEffect(() => {
    Promise.all([
      fetch('all-data.json').then(res => res.json()),
      fetch('git-graph.json').then(res => res.json())
    ]).then(([allDataJson, graphData]) => {
      const data = allDataJson as AllData;
      setAllData(data);

      // graphData is sorted New -> Old by backend, we want Old -> New
      const commitList = (graphData as CommitRecord[]).reverse();
      let cutIdx = 0;
      for (; cutIdx < commitList.length; cutIdx++) {
        if (data.commits[commitList[cutIdx].hash]) {
          break;
        }
      }
      setCommits(commitList.slice(cutIdx));

      // Select all machines by default
      setSelectedMachines(Object.keys(data.machines));
      setLoading(false);
    }).catch(err => {
      console.error("Failed to load data", err);
      setLoading(false);
    });
  }, []);

  // Commits on the selected branch (ancestor/descendant filtering)
  const filteredCommits = useMemo(() => {
    if (!selectedCommit) return commits;

    const parentMap = new Map<string, string[]>();
    const childrenMap = new Map<string, string[]>();

    commits.forEach(c => {
      parentMap.set(c.hash, c.parents);
      c.parents.forEach(p => {
        if (!childrenMap.has(p)) childrenMap.set(p, []);
        childrenMap.get(p)!.push(c.hash);
      });
    });

    const keepSet = new Set<string>();

    // Ancestors
    let stack = [selectedCommit];
    const visitedAncestors = new Set<string>();
    while (stack.length) {
      const h = stack.pop()!;
      if (visitedAncestors.has(h)) continue;
      visitedAncestors.add(h);
      keepSet.add(h);
      const parents = parentMap.get(h) || [];
      parents.forEach(p => stack.push(p));
    }

    // Descendants
    stack = [selectedCommit];
    const visitedDescendants = new Set<string>();
    while (stack.length) {
      const h = stack.pop()!;
      if (visitedDescendants.has(h)) continue;
      visitedDescendants.add(h);
      keepSet.add(h);
      const children = childrenMap.get(h) || [];
      children.forEach(c => stack.push(c));
    }

    return commits.filter(c => keepSet.has(c.hash));
  }, [commits, selectedCommit]);

  // Build benchmark chart data from allData (synchronous, no fetches)
  useEffect(() => {
    if (!allData || selectedMachines.length === 0 || filteredCommits.length === 0) {
      setBenchmarkData({});
      return;
    }

    const tempMap: Record<string, Record<string, Record<string, number>>> = {};
    const units: Record<string, string> = {};

    for (const commit of filteredCommits) {
      const commitData = allData.commits[commit.hash];
      if (!commitData) continue;

      for (const machine of selectedMachines) {
        const machineBenches = commitData.benchmarks[machine];
        if (!machineBenches) continue;

        for (const [benchId, val] of Object.entries(machineBenches)) {
          if (!tempMap[benchId]) tempMap[benchId] = {};
          if (!tempMap[benchId][commit.hash]) tempMap[benchId][commit.hash] = {};
          tempMap[benchId][commit.hash][machine] = val.estimate;
          units[benchId] = val.unit;
        }
      }
    }

    const newBenchmarkData: Record<string, any[]> = {};

    for (const benchId in tempMap) {
      const seriesData = filteredCommits.map(commit => {
        const dataPoint: any = {
          hash: commit.hash,
          shortHash: commit.hash.substring(0, 7),
          date: commit.date,
          message: commit.message,
        };

        if (tempMap[benchId][commit.hash]) {
          selectedMachines.forEach(machine => {
            if (tempMap[benchId][commit.hash][machine] !== undefined) {
              dataPoint[machine] = tempMap[benchId][commit.hash][machine];
            }
          });
        }
        return dataPoint;
      });

      (seriesData as any).unit = units[benchId];
      newBenchmarkData[benchId] = seriesData;
    }

    setBenchmarkData(newBenchmarkData);
  }, [allData, selectedMachines, filteredCommits]);

  const machines = useMemo(() => {
    if (!allData) return [];
    return Object.keys(allData.machines);
  }, [allData]);

  const chartGroups = useMemo(() => {
    return groupBy(Object.keys(benchmarkData), (key) => key.split('/')[0]);
  }, [benchmarkData]);

  const commitsWithData = useMemo(() => {
    if (!allData) return new Set<string>();
    return new Set(Object.keys(allData.commits));
  }, [allData]);

  const toggleMachine = (machine: string) => {
    setSelectedMachines(prev =>
      prev.includes(machine)
        ? prev.filter(m => m !== machine)
        : [...prev, machine]
    );
  };

  const getSystemInfo = (machine: string): SystemInfo | null => {
    if (!allData) return null;
    return allData.machines[machine] || null;
  };

  return {
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
  };
}
