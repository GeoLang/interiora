FROM rust:bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p interiora-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# a fresh named volume inherits this ownership, so the server can persist
# venue documents without running as root
RUN useradd -r -s /bin/false interiora && \
    mkdir -p /data && chown interiora:interiora /data

COPY --from=builder /app/target/release/interiora-server /usr/local/bin/interiora-server

USER interiora

ENV RUST_LOG=info,interiora=debug
ENV PORT=3000
ENV INTERIORA_DATA_DIR=/data

EXPOSE 3000
VOLUME /data

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["interiora-server"]
