FROM rust:1.73 AS builder
WORKDIR /usr/src/
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo install --path .

FROM debian:bookworm-slim
ARG APP_NAME=db-gateway
WORKDIR /workspace
COPY --from=builder /usr/local/cargo/bin/${APP_NAME} /usr/local/bin/app
RUN apt-get update && apt install -y openssl
ENTRYPOINT ["app"]
