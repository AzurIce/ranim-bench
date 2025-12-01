import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceLine } from 'recharts';
import { COLORS} from '../constants';

interface BenchmarkChartsProps {
  chartGroups: Record<string, string[]>;
  benchmarkData: Record<string, any[]>;
  selectedCommit: string | null;
  selectedMachines: string[];
  machines: string[];
}

export function BenchmarkCharts({
  chartGroups,
  benchmarkData,
  selectedCommit,
  selectedMachines,
  machines
}: BenchmarkChartsProps) {

  if (Object.keys(chartGroups).length === 0) {
    return (
      <div className="text-center py-20 bg-white rounded-xl border border-dashed border-gray-300">
        <p className="text-gray-500">Select machines to view benchmarks.</p>
      </div>
    );
  }

  return (
    <div className="space-y-10">
      {Object.entries(chartGroups).map(([groupName, keys]) => (
        <section key={groupName}>
          <h2 className="text-2xl font-bold mb-6 capitalize text-gray-800 flex items-center gap-2">
            <span className="w-2 h-8 bg-blue-600 rounded-full block"></span>
            {groupName}
          </h2>
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
            {keys.map(benchId => {
              const data = benchmarkData[benchId];
              const unit = (data as any).unit || 'ns';

              const formatY = (val: number) => {
                if (unit === 'ns') {
                  if (val > 1000000) return `${(val / 1000000).toFixed(1)}ms`;
                  if (val > 1000) return `${(val / 1000).toFixed(1)}µs`;
                  return `${val.toFixed(0)}ns`;
                }
                return val.toFixed(2);
              };

              return (
                <div key={benchId} className="bg-white p-4 rounded-xl border border-gray-200 shadow-sm hover:shadow-md transition-shadow">
                  <div className="mb-4 flex justify-between items-start">
                    <div>
                      <h3 className="font-bold text-gray-700 text-sm break-all">{benchId}</h3>
                      <p className="text-xs text-gray-400 mt-1">Lower is better</p>
                    </div>
                  </div>
                  <div className="h-[300px] w-full">
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={data} margin={{ top: 5, right: 20, bottom: 5, left: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f0f0f0" />
                        <XAxis
                          dataKey="shortHash"
                          tick={{ fontSize: 10, fill: '#9ca3af' }}
                          axisLine={false}
                          tickLine={false}
                        />
                        <YAxis
                          tickFormatter={formatY}
                          width={60}
                          tick={{ fontSize: 10, fill: '#9ca3af' }}
                          axisLine={false}
                          tickLine={false}
                        />
                        <Tooltip
                          contentStyle={{ borderRadius: '8px', border: 'none', boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.1)' }}
                          formatter={(val: number, name: string) => [formatY(val), name]}
                          labelFormatter={(label, payload) => {
                            if (payload && payload.length > 0) {
                              const d = payload[0].payload;
                              return `${d.date.split('T')[0]} (${label})\n${d.message}`;
                            }
                            return label;
                          }}
                        />
                        <Legend wrapperStyle={{ paddingTop: '10px', fontSize: '12px' }} />

                        {selectedCommit && (
                          <ReferenceLine x={selectedCommit.substring(0, 7)} stroke="#2563eb" strokeDasharray="3 3" />
                        )}

                        {selectedMachines.map((m) => {
                          const mIdx = machines.indexOf(m);
                          const color = COLORS[mIdx % COLORS.length];
                          return (
                            <Line
                              key={m}
                              type="monotone"
                              dataKey={m}
                              name={m}
                              stroke={color}
                              strokeWidth={2}
                              dot={{ r: 2, strokeWidth: 0 }}
                              activeDot={{ r: 5 }}
                              connectNulls={true}
                            />
                          );
                        })}
                      </LineChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
