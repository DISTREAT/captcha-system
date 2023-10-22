FROM alpine:3.18.4
USER root
RUN apk update && apk upgrade --no-cache
RUN apk add --no-cache rust cargo
RUN apk add --no-cache pkgconfig imagemagick-dev imagemagick-libs clang16-libclang font-liberation
RUN apk add --no-cache graphicsmagick-c++ # installing graphicsmagick should install common
                                          # dependencies needed in case of a different config

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

COPY src src
RUN touch src/main.rs  # Update file date
RUN cargo build --release
RUN cp /app/target/release/captcha-system /server

WORKDIR /
COPY config.toml .
RUN addgroup -S web && adduser -S web -G web
USER web
EXPOSE 80
CMD ["/server"]
