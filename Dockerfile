# Use the official Rust image as base
FROM rust:1.82-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    libxcb1-dev \
    libxcb-util0-dev \
    libxcb-ewmh-dev \
    libxcb-keysyms1-dev \
    libxcb-icccm4-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src
COPY build.rs ./

# Build the application
RUN cargo build --release

# Use a smaller base image for the final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libxcb1 \
    libxcb-util1 \
    libxcb-ewmh2 \
    libxcb-keysyms1 \
    libxcb-icccm4 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/wminspect /usr/local/bin/wminspect

# Create a non-root user
RUN useradd -m -s /bin/bash wminspect

# Switch to non-root user
USER wminspect

# Set the working directory
WORKDIR /home/wminspect

# Set the entrypoint
ENTRYPOINT ["wminspect"]
