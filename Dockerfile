FROM rust:1.75 as build-env
WORKDIR /app
ADD . /app
RUN apt-get -y update && apt-get install libudev-dev -y && \
    cargo build --release

FROM ubuntu:21.10
COPY --from=build-env /app/target/release/cds /usr/local/bin/cds

EXPOSE 8080 8081

CMD ["cds"]
