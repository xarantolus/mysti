# Build Stage
FROM rust:1-slim-buster as builder

RUN rustup target add armv5te-unknown-linux-gnueabi

RUN apt-get update && \
	apt-get install -y gcc-arm-linux-gnueabi libc6-dev-armel-cross

ENV CARGO_TARGET_ARMV5TE_UNKNOWN_LINUX_GNUEABI_LINKER=arm-linux-gnueabi-gcc

COPY . .

RUN cd server && cargo build --target armv5te-unknown-linux-gnueabi --release

# Final Stage
FROM arm32v5/debian:buster-slim

COPY --from=builder /target/armv5te-unknown-linux-gnueabi/release/mysti-server /mysti-server

ENV RUST_LOG=info

CMD ["/mysti-server"]
