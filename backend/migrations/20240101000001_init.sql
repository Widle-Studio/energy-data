CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS continents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    code VARCHAR(10) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS countries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    continent_id UUID REFERENCES continents(id),
    name VARCHAR(255) NOT NULL,
    iso_alpha2 VARCHAR(2) UNIQUE NOT NULL,
    iso_alpha3 VARCHAR(3) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS data_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL UNIQUE,
    url TEXT,
    description TEXT
);

CREATE TABLE IF NOT EXISTS indicators (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_id UUID REFERENCES data_sources(id),
    name VARCHAR(255) NOT NULL,
    code VARCHAR(100) UNIQUE NOT NULL,
    unit VARCHAR(50),
    category VARCHAR(100)
);

CREATE TABLE IF NOT EXISTS energy_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    country_id UUID NOT NULL REFERENCES countries(id),
    indicator_id UUID NOT NULL REFERENCES indicators(id),
    year INTEGER NOT NULL,
    value NUMERIC(20, 6) NOT NULL,
    UNIQUE (country_id, indicator_id, year)
);

CREATE INDEX IF NOT EXISTS idx_energy_data_chart
ON energy_data(country_id, indicator_id, year);

CREATE TABLE IF NOT EXISTS commodity_prices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    symbol VARCHAR(50) NOT NULL,
    price NUMERIC(12, 2) NOT NULL,
    percent_change NUMERIC(8, 2) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
