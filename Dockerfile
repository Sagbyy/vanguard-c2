# Builds the two backend binaries (vanguard-map + vanguard-control) in one image.
# The compose file picks which one to run per service via `command`.
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p vanguard-map -p vanguard-control

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/vanguard-map /usr/local/bin/vanguard-map
COPY --from=builder /app/target/release/vanguard-control /usr/local/bin/vanguard-control
# Overridden per service in docker-compose.yml.
CMD ["vanguard-map"]
