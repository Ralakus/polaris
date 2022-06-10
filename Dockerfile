FROM rust:1

WORKDIR /usr/src/polaris
COPY . .

RUN cargo install --path .
RUN mkdir /data

CMD ["polaris", "0.0.0.0:1965", "--cert", "polaris.voyage.cert", "--key", "polaris.voyage.key", "--data", "/data"]