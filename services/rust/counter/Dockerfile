# syntax=docker/dockerfile:1

# ---- Build stage: compile the counter binary from the workspace ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p counter_service

# ---- Runtime stage: minimal image with just the binary ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/counter_service /usr/local/bin/counter_service
EXPOSE 8000
ENV BIND_ADDR=0.0.0.0:8000
ENTRYPOINT ["counter_service"]
