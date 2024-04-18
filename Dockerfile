FROM rust:bookworm as builder
RUN mkdir /build
ADD . /build/
WORKDIR /build
RUN cargo build --release

FROM debian:bookworm
ENV DEBIAN_FRONTEND noninteractive
ENV LANG C.UTF-8
LABEL maintainer="shenjindi@ruzhila.cn"

COPY --from=builder /build/target/release/fgpt /bin/

ENTRYPOINT [ "/bin/fgpt"]