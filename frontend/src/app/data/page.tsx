'use client';

import { useState } from 'react';
import useSWR from 'swr';
import { DataFilters } from '../../components/filters/DataFilters';
import { EnergyChart } from '../../components/charts/EnergyChart';
import { DataTable } from '../../components/ui/DataTable';
import { getEnergyDataUrl, fetchEnergyData } from '../../lib/api';

export default function DataExplorer() {
  const [country, setCountry] = useState<string>('');
  const [indicator, setIndicator] = useState<string>('');

  const { data, isLoading } = useSWR(
    getEnergyDataUrl(country, indicator),
    () => fetchEnergyData(country, indicator)
  );

  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Data Explorer</h1>

      <DataFilters
        selectedCountry={country}
        setSelectedCountry={setCountry}
        selectedIndicator={indicator}
        setSelectedIndicator={setIndicator}
      />

      {isLoading ? (
        <div className="flex justify-center py-20">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
        </div>
      ) : (
        <>
          <EnergyChart data={data || []} />
          <DataTable data={data || []} />
        </>
      )}
    </div>
  );
}
