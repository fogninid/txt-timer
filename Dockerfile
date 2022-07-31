FROM rust as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:stable-slim
COPY --from=builder /usr/local/cargo/bin/txt-timer /usr/local/bin/txt-timer
ENTRYPOINT ["txt-timer"]
