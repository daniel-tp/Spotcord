FROM rust:alpine as builder
WORKDIR /usr/src

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl

RUN USER=root cargo new spotcord
WORKDIR /usr/src/spotcord
COPY Cargo.toml Cargo.lock ./
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

COPY src ./src
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:latest

COPY --from=builder /usr/src/spotcord/target/release/spotcord /usr/local/bin/spotcord

CMD ["spotcord"]