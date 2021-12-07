FROM rust:1.57-slim-buster AS builder
COPY . /build
WORKDIR /build
RUN cargo install --path . && cargo build --release

FROM debian:buster-slim
COPY --from=builder /build/target/release/idgend /apps/idgend
COPY etc /apps/etc

WORKDIR /apps
EXPOSE  7656
ENTRYPOINT ["/apps/idgend"]
