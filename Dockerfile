FROM rustlang/rust:nightly

RUN USER=root cargo new --bin grpc_gcp
WORKDIR /grpc_gcp
RUN cargo install cargo-tarpaulin
RUN cargo install cargo-watch

COPY ./Cargo.toml Cargo.toml
COPY ./Cargo.lock Cargo.lock
COPY ./build.rs build.rs
COPY ./proto proto
RUN cargo tarpaulin --no-run
RUN rm -r src

CMD ["cargo", "watch", "-x", "tarpaulin --verbose --out Html --output-dir /out"]
