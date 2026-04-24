'use client';

import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { DataRecord, ChartDataPoint } from '../../types';
import { useMemo } from 'react';

export function EnergyChart({ data }: { data: DataRecord[] }) {
  // Transform data into Recharts format and extract unique countries
  const { chartData, countries } = useMemo(() => {
    const map = new Map<number, ChartDataPoint>();
    const countrySet = new Set<string>();

    data.forEach(record => {
      let yearPoint = map.get(record.year);
      if (yearPoint === undefined) {
        yearPoint = { year: record.year };
        map.set(record.year, yearPoint);
      }
      yearPoint[record.country] = record.value;
      countrySet.add(record.country);
    });

    return {
      chartData: Array.from(map.values()).sort((a, b) => a.year - b.year),
      countries: Array.from(countrySet),
    };
  }, [data]);

  const colors = ['#2563eb', '#16a34a', '#dc2626', '#ca8a04', '#9333ea'];

  if (data.length === 0) {
    return <div className="h-[400px] flex items-center justify-center bg-gray-50 rounded-lg text-gray-500">No data available</div>;
  }

  return (
    <div className="h-[400px] w-full bg-white p-4 rounded-lg shadow">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="year" />
          <YAxis />
          <Tooltip />
          <Legend />
          {countries.map((country, idx) => (
            <Line
              key={country}
              type="monotone"
              dataKey={country}
              stroke={colors[idx % colors.length]}
              activeDot={{ r: 8 }}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
