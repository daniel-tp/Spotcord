FROM ekidd/rust-musl-builder:latest as builder
WORKDIR /usr/src

RUN USER=root cargo new spotcord
WORKDIR /usr/src/spotcord
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo build --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /usr/src/spotcord/target/x86_64-unknown-linux-musl/release/spotcord /usr/local/bin/spotcord

CMD ["spotcord"]