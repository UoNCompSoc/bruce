FROM rust:latest AS builder
RUN update-ca-certificates
RUN USER=root cargo new --bin bruce
WORKDIR ./bruce
COPY . ./
RUN cargo build --release
CMD ["/bruce/target/release/bruce"]

#FROM gcr.io/distroless/cc
FROM archlinux
USER 1000
WORKDIR /app
COPY --from=builder /bruce/target/release/bruce ./
VOLUME /data
CMD ["/app/bruce"]
