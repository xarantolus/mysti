# Build Stage
FROM rust:1-slim-buster as builder

COPY . .

RUN cd server && cargo build --release && cd .. && \
	cp target/release/mysti-server /mysti-server && \
	rm -rf target

# Final Stage
FROM debian:buster-slim

COPY --from=builder /mysti-server /mysti-server

ENV RUST_LOG=info

CMD ["/mysti-server"]
