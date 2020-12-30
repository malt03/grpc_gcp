FROM rustlang/rust:nightly

WORKDIR /usr/src/grpc_gcp
RUN cargo install cargo-tarpaulin
RUN cargo install cargo-watch

CMD ["cargo", "watch", "-x", "tarpaulin --verbose --out Html --output-dir ./out"]
