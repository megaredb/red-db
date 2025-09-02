FROM clux/muslrust:1.88.0-stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin red-db-server

FROM alpine AS runtime
RUN addgroup -S myuser && adduser -S myuser -G myuser
RUN mkdir /data && chown myuser:myuser /data
WORKDIR /data
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/red-db-server /usr/local/bin
USER myuser
COPY config.toml .
ENTRYPOINT ["/usr/local/bin/red-db-server"]