import type { DataRecord } from '../types/index.ts';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

const cache = new Map<string, Promise<DataRecord[]>>();

export async function fetchEnergyData(
  country?: string,
  indicator?: string,
  startYear?: number,
  endYear?: number
): Promise<DataRecord[]> {
  const params = new URLSearchParams();
  if (country) params.append('country', country);
  if (indicator) params.append('indicator', indicator);
  if (startYear) params.append('start_year', startYear.toString());
  if (endYear) params.append('end_year', endYear.toString());

  const url = `${API_URL}/api/v1/data?${params.toString()}`;

  if (cache.has(url)) {
    return cache.get(url)!;
  }

  const fetchPromise = (async () => {
    try {
      const res = await fetch(url);
      if (!res.ok) {
        throw new Error(`API error: ${res.status}`);
      }
      return await res.json();
    } catch (error) {
      console.error("Failed to fetch data:", error);
      cache.delete(url);
      return [];
    }
  })();

  cache.set(url, fetchPromise);
  return fetchPromise;
}

/**
 * Clears the API response cache.
 * Useful for testing or forcing a refresh.
 */
export function clearApiCache() {
  cache.clear();
}
