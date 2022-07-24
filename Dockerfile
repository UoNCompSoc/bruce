FROM docker.io/clux/muslrust:latest AS builder
WORKDIR /
RUN update-ca-certificates
RUN cargo new --bin bruce
WORKDIR ./bruce
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm -rd src target/x86_64-unknown-linux-musl/release/deps/bruce*
COPY src ./src
RUN cargo build --release

FROM docker.io/lsiobase/alpine:3.15
VOLUME /data
COPY root/ /
COPY --from=builder /bruce/target/x86_64-unknown-linux-musl/release/bruce /app/bruce
