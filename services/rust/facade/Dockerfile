# syntax=docker/dockerfile:1

# ---- Build stage: compile the facade binary from the workspace ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
# Cache downloaded crates and build artifacts between builds; the binary is
# copied out because the cached target dir is not part of the image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p facade_service \
    && cp target/release/facade_service /usr/local/bin/

# ---- Runtime stage: minimal image with just the binary ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /usr/local/bin/facade_service /usr/local/bin/facade_service
EXPOSE 8000
ENV BIND_ADDR=0.0.0.0:8000
ENTRYPOINT ["facade_service"]
