import { useState, useEffect, useMemo } from 'react';
import { groupBy } from 'lodash';
import { CommitRecord, DbManifest, RunManifest, BenchmarkResult, SystemInfo } from '../types';

export function useAppData() {
  const [commits, setCommits] = useState<CommitRecord[]>([]);
  const [dbManifest, setDbManifest] = useState<Map<string, string[]>>(new Map());
  const [runManifests, setRunManifests] = useState<Record<string, RunManifest>>({}); // Key: hash/machine
  const [selectedMachines, setSelectedMachines] = useState<string[]>([]);
  const [benchmarkData, setBenchmarkData] = useState<Record<string, any[]>>({});
  const [loading, setLoading] = useState(true);
  const [selectedCommit, setSelectedCommit] = useState<string | null>(null);

  // Load metadata
  useEffect(() => {
    Promise.all([
      fetch('db/db.json').then(res => res.json()),
      fetch('git-graph.json').then(res => res.json())
    ]).then(([dbData, graphData]) => {
      const db = dbData as DbManifest;
      const manifestMap = new Map(Object.entries(db.benches));
      setDbManifest(manifestMap);

      // graphData is sorted New -> Old by backend, but we want Old -> New
      const commits = (graphData as CommitRecord[]).reverse();
      let cutIdx = 0;
      for (; cutIdx < commits.length; cutIdx++) {
        if (manifestMap.has(commits[cutIdx].hash)) {
          break
        }
      }
      setCommits(commits.slice(cutIdx, commits.length));

      // Extract all unique machines
      const allMachines = new Set<string>();
      manifestMap.forEach(runs => runs.forEach(r => allMachines.add(r)));
      const machineList = Array.from(allMachines);

      // if (machineList.length > 0) {
      //   setSelectedMachines([machineList[0]]);
      // }
      setSelectedMachines(machineList);
      setLoading(false);
    }).catch(err => {
      console.error("Failed to load metadata", err);
      setLoading(false);
    });
  }, []);

  // Commits on the selected branch
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

  // Load benchmark data when selection changes
  useEffect(() => {
    if (selectedMachines.length === 0 || filteredCommits.length === 0 || dbManifest.size === 0) {
      setBenchmarkData({});
      return;
    }

    async function loadData() {
      const tempMap: Record<string, Record<string, Record<string, number>>> = {}; // benchId -> hash -> machine -> value
      const units: Record<string, string> = {};
      const newRunManifests: Record<string, RunManifest> = { ...runManifests };

      const tasks: Promise<void>[] = [];

      for (const commit of filteredCommits) {
        const runs = dbManifest.get(commit.hash);
        if (!runs) continue;

        for (const machine of selectedMachines) {
          if (runs.includes(machine)) {
            const runKey = `${commit.hash}/${machine}`;

            tasks.push((async () => {
              // 1. Load Run Manifest if needed
              let manifest = newRunManifests[runKey];
              if (!manifest) {
                try {
                  const res = await fetch(`db/${commit.hash}/${machine}/run.json`);
                  manifest = await res.json();
                  newRunManifests[runKey] = manifest;
                } catch (e) {
                  // console.warn(`Failed to load run.json for ${runKey}`, e);
                  return;
                }
              }

              // 2. Load Benchmarks
              const benchTasks = manifest.benchmarks.map(async (benchId) => {
                try {
                  const res = await fetch(`db/${commit.hash}/${machine}/${benchId}.json`);
                  const json: BenchmarkResult = await res.json();

                  if (json.mean && json.mean.estimate) {
                    if (!tempMap[benchId]) tempMap[benchId] = {};
                    if (!tempMap[benchId][commit.hash]) tempMap[benchId][commit.hash] = {};
                    tempMap[benchId][commit.hash][machine] = json.mean.estimate;
                    units[benchId] = json.mean.unit;
                  }
                } catch (e) {
                  console.warn(`Failed to load bench ${benchId}`, e);
                }
              });
              await Promise.all(benchTasks);
            })());
          }
        }
      }

      await Promise.all(tasks);

      setRunManifests(newRunManifests);

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
      // console.log(newBenchmarkData)

      setBenchmarkData(newBenchmarkData);
    }

    loadData();
  }, [selectedMachines, dbManifest, filteredCommits]); 
  // Note: runManifests is excluded from dependencies as in original code to avoid loops

  const machines = useMemo(() => {
    const set = new Set<string>();
    dbManifest.forEach(runs => runs.forEach(r => set.add(r)));
    return Array.from(set);
  }, [dbManifest]);

  const chartGroups = useMemo(() => {
    return groupBy(Object.keys(benchmarkData), (key) => key.split('/')[0]);
  }, [benchmarkData]);

  const commitsWithData = useMemo(() => {
    const s = new Set<string>();
    for (const hash of dbManifest.keys()) {
      s.add(hash);
    }
    return s;
  }, [dbManifest]);

  const toggleMachine = (machine: string) => {
    setSelectedMachines(prev =>
      prev.includes(machine)
        ? prev.filter(m => m !== machine)
        : [...prev, machine]
    );
  };

  const getSystemInfo = (machine: string): SystemInfo | null => {
    // Find the latest commit for this machine that has a loaded manifest
    for (let i = commits.length - 1; i >= 0; i--) {
      const commit = commits[i];
      const runKey = `${commit.hash}/${machine}`;
      if (runManifests[runKey]) {
        return runManifests[runKey].system;
      }
    }
    return null;
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
