# syntax=docker/dockerfile:1

# ---- Build stage: compile the facade binary from the workspace ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p facade_service

# ---- Runtime stage: minimal image with just the binary ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/facade_service /usr/local/bin/facade_service
EXPOSE 8000
ENV BIND_ADDR=0.0.0.0:8000
ENTRYPOINT ["facade_service"]
