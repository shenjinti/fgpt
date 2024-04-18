FROM rust:alpine3.19 as builder
RUN mkdir /build
ADD . /build/
WORKDIR /build
RUN cargo build --release

FROM alpine:3.19
ENV DEBIAN_FRONTEND noninteractive
ENV LANG C.UTF-8

COPY --from=builder /build/target/release/fgpt /bin/

ENTRYPOINT [ "/bin/fgpt"]