FROM rust:latest AS builder
RUN update-ca-certificates
RUN USER=root cargo new --bin bruce
WORKDIR ./bruce
COPY . ./
RUN cargo build --release
CMD ["/bruce/target/release/bruce"]

#FROM gcr.io/distroless/cc
FROM archlinux
WORKDIR /app
VOLUME /data
COPY --from=builder /bruce/target/release/bruce ./
CMD ["/app/bruce"]
