FROM rust:1.76-slim

# Install build dependencies
RUN apt-get update && apt-get install -y \
    git \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack for WebAssembly tooling
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Set working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock to cache dependencies
COPY Cargo.toml ./
RUN mkdir -p src && \
    touch src/lib.rs && \
    cargo fetch

# Copy the rest of the code
COPY . .

# Verify compilation
RUN cargo check

# Default command to run tests
CMD ["cargo", "test"] 