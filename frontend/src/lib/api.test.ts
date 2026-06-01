import test from 'node:test';
import assert from 'node:assert';
import { fetchEnergyData } from './api.ts';

test('fetchEnergyData calls fetch with correct URL when all params are provided', async (t) => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const fetchMock = t.mock.method(global, 'fetch', async (url: string | URL, _options?: RequestInit) => {
    assert.ok(url.toString().includes('country=US'));
    assert.ok(url.toString().includes('indicator=EG.ELC.ACCS.ZS'));
    assert.ok(url.toString().includes('start_year=2010'));
    assert.ok(url.toString().includes('end_year=2020'));
    return {
      ok: true,
      json: async () => [{ country: 'US', indicator: 'EG.ELC.ACCS.ZS', year: 2010, value: 100 }]
    } as Response;
  });

  const data = await fetchEnergyData('US', 'EG.ELC.ACCS.ZS', 2010, 2020);
  assert.strictEqual(data.length, 1);
  assert.strictEqual(data[0].country, 'US');
  assert.strictEqual(fetchMock.mock.callCount(), 1);
});

test('fetchEnergyData calls fetch with correct URL when no params are provided', async (t) => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const fetchMock = t.mock.method(global, 'fetch', async (url: string | URL, _options?: RequestInit) => {
    assert.strictEqual(url, 'http://localhost:8080/api/v1/data?');
    return {
      ok: true,
      json: async () => []
    } as Response;
  });

  const data = await fetchEnergyData();
  assert.strictEqual(data.length, 0);
  assert.strictEqual(fetchMock.mock.callCount(), 1);
});

test('fetchEnergyData returns empty array and logs error when response is not ok', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  t.mock.method(global, 'fetch', async (_url: string | URL, _options?: RequestInit) => {
    return {
      ok: false,
      status: 500,
    } as Response;
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof Error);
  assert.strictEqual((args[1] as Error).message, 'API error: 500');
});

test('fetchEnergyData returns empty array and logs error when fetch throws', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  t.mock.method(global, 'fetch', async () => {
    throw new Error('Network failure');
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof Error);
  assert.strictEqual((args[1] as Error).message, 'Network failure');
});

test('fetchEnergyData returns empty array and logs error when res.json() throws', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  t.mock.method(global, 'fetch', async () => {
    return {
      ok: true,
      json: async () => {
        throw new SyntaxError('Unexpected token');
      }
    } as Response;
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof SyntaxError);
  assert.strictEqual((args[1] as Error).message, 'Unexpected token');
});

test('fetchEnergyData returns empty array and logs error when response is 404 Not Found', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  t.mock.method(global, 'fetch', async (_url: string | URL, _options?: RequestInit) => {
    return {
      ok: false,
      status: 404,
    } as Response;
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof Error);
  assert.strictEqual((args[1] as Error).message, 'API error: 404');
});

test('fetchEnergyData returns empty array and logs error when response is 401 Unauthorized', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  t.mock.method(global, 'fetch', async (_url: string | URL, _options?: RequestInit) => {
    return {
      ok: false,
      status: 401,
    } as Response;
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof Error);
  assert.strictEqual((args[1] as Error).message, 'API error: 401');
});

test('fetchEnergyData returns empty array and logs error when fetch throws a non-Error object (e.g. string)', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  t.mock.method(global, 'fetch', async () => {
    throw 'String error thrown';
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.strictEqual(args[1], 'String error thrown');
});

test('fetchEnergyData returns empty array and logs error when fetch throws an AbortError', async (t) => {
  const consoleSpy = t.mock.method(console, 'error', () => {});

  t.mock.method(global, 'fetch', async () => {
    const error = new Error('The operation was aborted');
    error.name = 'AbortError';
    throw error;
  });

  const data = await fetchEnergyData('US');
  assert.strictEqual(data.length, 0);
  assert.strictEqual(consoleSpy.mock.callCount(), 1);
  const args = consoleSpy.mock.calls[0].arguments;
  assert.strictEqual(args[0], 'Failed to fetch data:');
  assert.ok(args[1] instanceof Error);
  assert.strictEqual((args[1] as Error).name, 'AbortError');
  assert.strictEqual((args[1] as Error).message, 'The operation was aborted');
});
