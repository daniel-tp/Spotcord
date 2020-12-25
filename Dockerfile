FROM ekidd/rust-musl-builder:latest as builder

RUN USER=rust cargo new spotcord
WORKDIR ./spotcord
#ADD --chown=rust:rust Cargo.toml Cargo.lock ./
#RUN cargo build --release

ADD --chown=rust:rust . ./
RUN cargo build --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /home/rust/src/spotcord/target/x86_64-unknown-linux-musl/release/spotcord /usr/local/bin/spotcord

ENTRYPOINT spotcord