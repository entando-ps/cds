FROM rust:1.89 as build-env
WORKDIR /app
ADD . /app
RUN cargo build --release && \
mkdir -p /entando-data/public && mkdir -p /entando-data/protected

FROM gcr.io/distroless/cc
COPY --from=build-env /app/target/release/cds /

EXPOSE 8080 8081

CMD ["./cds"]
