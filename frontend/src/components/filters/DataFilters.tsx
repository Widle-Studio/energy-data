'use client';

import React from 'react';

export function DataFilters({
  selectedCountry,
  setSelectedCountry,
  selectedIndicator,
  setSelectedIndicator
}: {
  selectedCountry: string;
  setSelectedCountry: (v: string) => void;
  selectedIndicator: string;
  setSelectedIndicator: (v: string) => void;
}) {
  return (
    <div className="flex gap-4 p-4 bg-white shadow rounded-lg mb-6">
      <div className="flex-1">
        <label className="block text-sm font-medium text-gray-700 mb-1">Country</label>
        <select
          value={selectedCountry}
          onChange={(e) => setSelectedCountry(e.target.value)}
          className="w-full border-gray-300 rounded-md shadow-sm p-2 border focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">All Countries</option>
          <option value="US">United States (US)</option>
          <option value="DE">Germany (DE)</option>
          <option value="CN">China (CN)</option>
          <option value="IN">India (IN)</option>
        </select>
      </div>

      <div className="flex-1">
        <label className="block text-sm font-medium text-gray-700 mb-1">Indicator</label>
        <select
          value={selectedIndicator}
          onChange={(e) => setSelectedIndicator(e.target.value)}
          className="w-full border-gray-300 rounded-md shadow-sm p-2 border focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">All Indicators</option>
          <option value="EG.USE.ELEC.KH.PC">Electricity Power Consumption (kWh per capita)</option>
        </select>
      </div>
    </div>
  );
}
