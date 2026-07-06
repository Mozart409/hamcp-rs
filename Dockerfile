# syntax=docker/dockerfile:1

# ---------- builder ----------
FROM rust:1.92-alpine3.20 AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static ca-certificates

ENV OPENSSL_STATIC=1 \
    OPENSSL_NO_VENDOR=1

# -- Cache dependency build --
COPY Cargo.toml Cargo.lock ./
COPY crates/common/mcp-common/Cargo.toml ./crates/common/mcp-common/Cargo.toml
COPY crates/homeassistant-mcp/hamcp/Cargo.toml ./crates/homeassistant-mcp/hamcp/Cargo.toml
COPY crates/homeassistant-mcp/hamcp-server/Cargo.toml ./crates/homeassistant-mcp/hamcp-server/Cargo.toml

RUN mkdir -p crates/common/mcp-common/src \
    && mkdir -p crates/homeassistant-mcp/hamcp/src/models \
    && mkdir -p crates/homeassistant-mcp/hamcp-server/src \
    && echo 'fn main() {}' > crates/homeassistant-mcp/hamcp-server/src/main.rs \
    && echo '' > crates/common/mcp-common/src/lib.rs \
    && echo '' > crates/homeassistant-mcp/hamcp/src/lib.rs \
    && cargo build --release --locked \
    && rm -rf crates/common/mcp-common/src \
    && rm -rf crates/homeassistant-mcp/hamcp/src \
    && rm -rf crates/homeassistant-mcp/hamcp-server/src

# -- Build the real application --
COPY crates ./crates

RUN touch crates/homeassistant-mcp/hamcp-server/src/main.rs \
    && cargo build --release --locked --bin hamcp-server \
    && strip /app/target/release/hamcp-server

# ---------- runtime ----------
FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/release/hamcp-server /hamcp-server
COPY --from=builder /app/Cargo.lock /Cargo.lock

LABEL org.opencontainers.image.title="hamcp" \
    org.opencontainers.image.description="MCP server for Home Assistant" \
    org.opencontainers.image.source="https://github.com/mozart409/hamcp-rs" \
    org.opencontainers.image.licenses="MIT"

EXPOSE 3000

USER 65532:65532

ENTRYPOINT ["/hamcp-server"]
