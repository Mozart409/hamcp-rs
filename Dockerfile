# syntax=docker/dockerfile:1

# ---------- builder ----------
FROM rust:1.92-alpine3.20 AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static ca-certificates

ENV OPENSSL_STATIC=1 \
    OPENSSL_NO_VENDOR=1

# -- Cache dependency build --
# Copy only manifests and create a dummy lib/main so `cargo build` compiles
# dependencies without the real source. This layer is cached until
# Cargo.toml or Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
COPY mcp/Cargo.toml ./mcp/Cargo.toml

RUN mkdir -p mcp/src \
    && echo 'fn main() {}' > mcp/src/main.rs \
    && echo '' > mcp/src/lib.rs \
    && cargo build --release --locked \
    && rm -rf mcp/src

# -- Build the real application --
COPY mcp/src ./mcp/src

# Touch main.rs so cargo detects it changed (timestamps may match the dummy)
RUN touch mcp/src/main.rs mcp/src/lib.rs \
    && cargo build --release --locked \
    && strip /app/target/release/mcp

# ---------- runtime ----------
FROM scratch

# Bring in CA certificates for HTTPS
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the statically-linked binary
COPY --from=builder /app/target/release/mcp /mcp

# Copy Cargo.lock so Trivy can scan Rust dependencies
COPY --from=builder /app/Cargo.lock /Cargo.lock

# OCI image labels
LABEL org.opencontainers.image.title="hamcp" \
    org.opencontainers.image.description="MCP server for Home Assistant" \
    org.opencontainers.image.source="https://github.com/mozart409/hamcp-rs" \
    org.opencontainers.image.licenses="MIT"

EXPOSE 3000

USER 65532:65532

ENTRYPOINT ["/mcp"]
