import { useMemo, useState } from 'react';
import { CommitRecord } from '../types';

interface CommitGraphProps {
  commits: CommitRecord[];
  selectedCommit: string | null;
  onCommitSelect: (hash: string) => void;
  commitsWithData: Set<string>;
}

interface Node {
  x: number;
  y: number;
  hash: string;
  color: string;
  lane: number;
}

interface Edge {
  p1: { x: number; y: number };
  p2: { x: number; y: number };
  color: string;
}

const LANE_HEIGHT = 40;
const X_SPACING = 50;
const NODE_RADIUS = 6;
// Fallback colors if not provided
const DEFAULT_COLORS = ["#e11d48", "#2563eb", "#16a34a", "#d97706", "#9333ea", "#0891b2"];

export function CommitGraph({ commits, selectedCommit, onCommitSelect, commitsWithData }: CommitGraphProps) {
  const { nodes, edges, width, height, laneLabels } = useMemo(() => {
    // Filter commits: find the oldest commit that has data, and cut off everything before it.
    // Commits are Newest -> Oldest.
    let cutoffIndex = commits.length - 1;
    for (let i = commits.length - 1; i >= 0; i--) {
        if (commitsWithData.has(commits[i].hash)) {
            cutoffIndex = i;
            break;
        }
    }
    // Slice keeps 0 to cutoffIndex (inclusive)
    const filteredCommits = commits.slice(0, cutoffIndex + 1);

    return processCommits(filteredCommits);
  }, [commits, commitsWithData]);

  const [hoveredHash, setHoveredHash] = useState<string | null>(null);

  return (
    <div className="flex relative w-full border border-gray-200 rounded-lg bg-white" style={{ height: height + 20 }}>
       {/* Sticky Lane Labels */}
       <div className="sticky left-0 z-10 bg-white/90 backdrop-blur-sm border-r border-gray-200 shrink-0 flex flex-col shadow-sm rounded-l-lg" 
            style={{ paddingTop: 5, width: 200 }}>
          {Array.from({ length: Math.ceil(height / LANE_HEIGHT) }).map((_, i) => {
              const label = laneLabels.get(i);
              return (
                <div key={i} className="h-[40px] flex items-center px-3 text-xs font-medium text-gray-600 whitespace-nowrap overflow-hidden text-ellipsis" title={label}>
                   {label || ''}
                </div>
              );
          })}
       </div>

       {/* Graph Area */}
       <div className="overflow-x-auto flex-1 rounded-r-lg">
           <svg width={width} height={height + 20} className="block font-mono text-[10px]">
             {/* Edges */}
             {edges.map((e, i) => (
               <path 
                 key={`e-${i}`} 
                 d={`M ${e.p1.x} ${e.p1.y} C ${e.p1.x + X_SPACING/2} ${e.p1.y}, ${e.p2.x - X_SPACING/2} ${e.p2.y}, ${e.p2.x} ${e.p2.y}`}
                 stroke={e.color}
                 strokeWidth={2}
                 fill="none"
                 opacity={0.6}
               />
             ))}
             
             {/* Nodes */}
             {nodes.map((n) => {
               const hasData = commitsWithData.has(n.hash);
               const isSelected = selectedCommit === n.hash;
               const isHovered = hoveredHash === n.hash;
               
               const nodeColor = hasData ? n.color : "#e5e7eb";
               const strokeColor = hasData ? n.color : "#9ca3af";
               
               return (
                 <g 
                   key={n.hash} 
                   onClick={() => onCommitSelect(n.hash)} 
                   onMouseEnter={() => setHoveredHash(n.hash)}
                   onMouseLeave={() => setHoveredHash(null)}
                   className="cursor-pointer group"
                 >
                   <circle 
                     cx={n.x} 
                     cy={n.y} 
                     r={isHovered ? NODE_RADIUS + 2 : NODE_RADIUS} 
                     fill={isSelected ? "#fff" : nodeColor}
                     stroke={strokeColor}
                     strokeWidth={isSelected ? 3 : (hasData ? 0 : 2)}
                     strokeDasharray={hasData ? "none" : "3 2"}
                     className="transition-all duration-200"
                   />
                   
                   {isSelected && (
                      <circle cx={n.x} cy={n.y} r={NODE_RADIUS + 4} fill="none" stroke={strokeColor} strokeWidth={1} opacity={0.5} />
                   )}

                   {/* Hash Label */}
                   <text 
                     x={n.x} 
                     y={n.y + 20} 
                     textAnchor="middle" 
                     className={`text-[9px] ${isSelected ? 'font-bold fill-blue-600' : 'fill-gray-500'}`}
                   >
                     {n.hash.substring(0, 7)}
                   </text>

                   {/* Invisible hit target for better usability */}
                   <circle cx={n.x} cy={n.y} r={NODE_RADIUS + 10} fill="transparent" />
                 </g>
               );
             })}
           </svg>
       </div>
    </div>
  );
}

function processCommits(commits: CommitRecord[]) {
    const hashToNode = new Map<string, Node>();
    const nodes: Node[] = [];
    const edges: Edge[] = [];
    const laneLabels = new Map<number, string>();
    let maxLane = 0;
    
    // From old to new
    for (let i = 0; i < commits.length; i++) {
        const commit = commits[i];
        const lane = commit.column ?? 0;
        maxLane = Math.max(maxLane, lane);
        
        const x = i * X_SPACING + 20;
        const y = lane * LANE_HEIGHT + LANE_HEIGHT / 2 + 5;
        
        // Resolve color: use provided color or fallback based on lane
        let color = commit.color;
        if (!color) {
            color = DEFAULT_COLORS[lane % DEFAULT_COLORS.length];
        } else {
             color = mapColor(color);
        }
        
        const node: Node = { x, y, hash: commit.hash, color, lane };
        nodes.push(node);
        hashToNode.set(commit.hash, node);
        
        // Update lane labels
        if (commit.branches && commit.branches.length > 0) {
             const existing = laneLabels.get(lane);
             // Dedup labels
             const currentLabels = existing ? existing.split(', ') : [];
             const newLabels = commit.branches.filter(b => !currentLabels.includes(b));
             if (newLabels.length > 0) {
                 laneLabels.set(lane, existing ? `${existing}, ${newLabels.join(', ')}` : newLabels.join(', '));
             }
        }
    }

    // Edges
    
    for (const commit of commits) {
        const childNode = hashToNode.get(commit.hash);
        if (!childNode) continue;
        
        for (const pHash of commit.parents) {
            const parentNode = hashToNode.get(pHash);
            if (parentNode) {
                // Edge Parent -> Child
                edges.push({
                    p1: { x: parentNode.x, y: parentNode.y },
                    p2: { x: childNode.x, y: childNode.y },
                    color: parentNode.color // Use parent's color for the line
                });
            }
        }
    }

    return {
        nodes,
        edges,
        width: commits.length * X_SPACING + 40,
        height: (maxLane + 1) * LANE_HEIGHT + 20,
        laneLabels
    };
}

function mapColor(name: string): string {
    const map: Record<string, string> = {
        "red": "#ef4444",
        "green": "#22c55e",
        "blue": "#3b82f6",
        "yellow": "#eab308",
        "orange": "#f97316",
        "purple": "#a855f7",
        "cyan": "#06b6d4",
        "magenta": "#d946ef",
        "white": "#6b7280", // Gray for white
        "gray": "#9ca3af",
        "turquoise": "#14b8a6"
    };
    return map[name.toLowerCase()] || name;
}
