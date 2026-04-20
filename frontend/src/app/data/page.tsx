'use client';

import { useState, useEffect } from 'react';
import { DataFilters } from '../../components/filters/DataFilters';
import { EnergyChart } from '../../components/charts/EnergyChart';
import { DataTable } from '../../components/ui/DataTable';
import { fetchEnergyData } from '../../lib/api';
import { DataRecord } from '../../types';

export default function DataExplorer() {
  const [data, setData] = useState<DataRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [country, setCountry] = useState<string>('');
  const [indicator, setIndicator] = useState<string>('');

  useEffect(() => {
    async function loadData() {
      setLoading(true);
      const records = await fetchEnergyData(country, indicator);
      setData(records);
      setLoading(false);
    }
    loadData();
  }, [country, indicator]);

  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Data Explorer</h1>

      <DataFilters
        selectedCountry={country}
        setSelectedCountry={setCountry}
        selectedIndicator={indicator}
        setSelectedIndicator={setIndicator}
      />

      {loading ? (
        <div className="flex justify-center py-20">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
        </div>
      ) : (
        <>
          <EnergyChart data={data} />
          <DataTable data={data} />
        </>
      )}
    </div>
  );
}
