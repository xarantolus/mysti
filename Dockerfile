# Dockerfile that tries to minimize the size of the image,
# downloading dependencies in the first stage, building in the second and running in the second stage
# It must work on both x86_64 and arm64 architectures
FROM rust:1.74-slim-buster as deps

# Download dependencies
RUN cargo new --bin server
RUN cargo new --lib common

COPY server/Cargo.toml server/Cargo.toml
COPY client/Cargo.toml client/Cargo.toml
COPY common/Cargo.toml common/Cargo.toml
COPY Cargo.lock ./Cargo.lock
COPY Cargo.toml ./Cargo.toml

RUN cd server && \
	cargo build --release && \
	rm -rf target/release/deps/server* server/src common/src

FROM rust:1.74.0 as builder

# Copy the dependencies cache
COPY --from=deps /usr/local/cargo /usr/local/cargo
COPY --from=deps /target /target

COPY --from=deps server/Cargo.toml server/Cargo.toml
COPY --from=deps common/Cargo.toml common/Cargo.toml

# Build the project
COPY server/src ./server/src
COPY common/src ./common/src

RUN cd server && cargo build --release

# Run the project
FROM debian:buster-slim

COPY --from=builder /target/release/mysti-server /mysti-server

CMD ["/mysti-server"]

