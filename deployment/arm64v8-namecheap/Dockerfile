# Build Stage
FROM rust:1-slim-buster as builder

RUN rustup target add aarch64-unknown-linux-gnu

RUN apt-get update && \
	apt-get install -y g++-aarch64-linux-gnu libc6-dev-arm64-cross

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

COPY . .

RUN cd server && cargo build --target aarch64-unknown-linux-gnu --release

# Final Stage
FROM arm64v8/debian:buster-slim

COPY --from=builder /target/aarch64-unknown-linux-gnu/release/mysti-server /mysti-server

ENV RUST_LOG=info

CMD ["/mysti-server"]
