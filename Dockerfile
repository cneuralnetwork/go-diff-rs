FROM rust:1.85.0-bookworm AS builder

WORKDIR /source
COPY . .
RUN sha256sum --check tests/original/SHA256SUMS \
    && cargo build --release --locked --bin go-diff-rs

FROM debian:bookworm-slim

COPY --from=builder /source/target/release/go-diff-rs /usr/local/bin/go-diff-rs
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/go-diff-rs"]
CMD ["--help"]
