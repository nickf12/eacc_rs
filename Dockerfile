# Builder stage
FROM rust:1.86.0 AS builder

WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
# ENV SQLX_OFFLINE=true
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Install OpenSSL - it is  dynamicallylinked by some of our dependencies
# Install ca-certificates - it is needed to verify TLS certificates
# when establishing HTTPS connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/eacc_rs eacc_rs
# Copy the media folder
COPY --from=builder /app/media /app/media

EXPOSE 3000
ENTRYPOINT [ "./eacc_rs" ]