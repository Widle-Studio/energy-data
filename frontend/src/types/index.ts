export interface DataRecord {
  country: string;
  indicator: string;
  year: number;
  value: number;
}

export interface ChartDataPoint {
  year: number;
  [country: string]: number;
}
