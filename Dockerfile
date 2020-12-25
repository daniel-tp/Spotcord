FROM ekidd/rust-musl-builder:latest as builder
WORKDIR /usr/src

RUN USER=rust cargo new spotcord
WORKDIR /usr/src/spotcord
ADD --chown=rust:rust Cargo.toml Cargo.lock ./
RUN cargo build --release

ADD --chown=rust:rust src ./src
RUN cargo build --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /usr/src/spotcord/target/x86_64-unknown-linux-musl/release/spotcord /usr/local/bin/spotcord

CMD ["spotcord"]