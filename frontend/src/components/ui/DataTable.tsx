'use client';

import { DataRecord } from '../../types';

export function DataTable({ data }: { data: DataRecord[] }) {
  if (data.length === 0) {
    return <div className="p-4 text-center text-gray-500">No data records found.</div>;
  }

  return (
    <div className="overflow-x-auto bg-white rounded-lg shadow mt-6">
      <table className="min-w-full divide-y divide-gray-200">
        <thead className="bg-gray-50">
          <tr>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Year</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Country</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Indicator</th>
            <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">Value</th>
          </tr>
        </thead>
        <tbody className="bg-white divide-y divide-gray-200">
          {data.map((row, idx) => (
            <tr key={idx}>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{row.year}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{row.country}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{row.indicator}</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 text-right">
                {row.value.toLocaleString(undefined, { maximumFractionDigits: 2 })}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
