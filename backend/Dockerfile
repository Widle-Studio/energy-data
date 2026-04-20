# Build stage
FROM rust:1.75-bullseye as builder

WORKDIR /usr/src/app

# Create a dummy project to build dependencies
RUN USER=root cargo new backend
WORKDIR /usr/src/app/backend
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Update timestamps to force a rebuild of the application code
RUN touch src/main.rs

# Build the real application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Install runtime dependencies (e.g., OpenSSL, ca-certificates for reqwest)
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment
COPY --from=builder /usr/src/app/backend/target/release/energtx-backend /app/energtx-backend

# Explicitly expose port 8080 (Cloud Run default)
EXPOSE 8080

# Run the binary
CMD ["./energtx-backend"]
