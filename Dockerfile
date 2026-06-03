# syntax=docker/dockerfile:1

FROM rust:1.86-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false dietopt

WORKDIR /app
COPY --from=builder /app/target/release/diet_optimizer /usr/local/bin/diet_optimizer

USER dietopt
EXPOSE 8080
ENV HOST=0.0.0.0
ENV PORT=8080
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/diet_optimizer"]
