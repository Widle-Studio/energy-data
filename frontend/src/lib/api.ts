import type { DataRecord } from '../types/index.ts';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export const getEnergyDataUrl = (
  country?: string,
  indicator?: string,
  startYear?: number,
  endYear?: number
) => {
  const params = new URLSearchParams();
  if (country) params.append('country', country);
  if (indicator) params.append('indicator', indicator);
  if (startYear) params.append('start_year', startYear.toString());
  if (endYear) params.append('end_year', endYear.toString());

  return `${API_URL}/api/v1/data?${params.toString()}`;
}

export async function fetchEnergyData(
  country?: string,
  indicator?: string,
  startYear?: number,
  endYear?: number
): Promise<DataRecord[]> {
  const url = getEnergyDataUrl(country, indicator, startYear, endYear);

  try {
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(`API error: ${res.status}`);
    }
    return await res.json();
  } catch (error) {
    console.error("Failed to fetch data:", error);
    return [];
  }
}
