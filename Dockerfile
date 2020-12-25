FROM rust:alpine as builder
WORKDIR /usr/src

RUN USER=root cargo new spotcord
WORKDIR /usr/src/spotcord
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo build --release

FROM alpine:latest

COPY --from=builder /usr/src/spotcord/target/release/spotcord /usr/local/bin/spotcord

CMD ["spotcord"]