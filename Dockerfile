# ------------------------------------------------------------------------------
# Project:     Ferrox
# Source:      https://github.com/Res-NeoTech/ferrox
# Description: Ultra-lightweight, natively compiled Alpine image.
# Details:     Uses a multi-stage build to compile the Rust binary.
# Author:      FauZaPespi
# ------------------------------------------------------------------------------

# Stage 1: Build the binary natively on Alpine
FROM alpine:3.23 AS builder

# Install Rust, Cargo, and build tools
RUN apk add --no-cache \
    rust \
    cargo \
    build-base \
    git \
    openssl-dev

# Clone the repository and build the project
WORKDIR /build
RUN git clone https://github.com/Res-NeoTech/ferrox.git . 
RUN cargo build --release

# Stage 2: Create the final, ultra-lightweight image
FROM alpine:3.23

# Install only the runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app
COPY --from=builder /build/target/release/ferrox /app/ferrox
COPY ferrox-compose.yml /app/ferrox-compose.yml

# Run it
ENTRYPOINT ["/app/ferrox"]