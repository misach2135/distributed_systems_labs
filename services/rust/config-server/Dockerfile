# syntax=docker/dockerfile:1

# ---- Build stage: compile the config-server binary from the workspace ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p config_server

# ---- Runtime stage: minimal image with just the binary ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/config_server /usr/local/bin/config_server
EXPOSE 8000
ENV BIND_ADDR=0.0.0.0:8000
ENTRYPOINT ["config_server"]
