# FROM rust:latest as build
FROM arm32v7/rust:latest as build

ARG API_KEY
ENV API_KEY ${API_KEY}

RUN user=root cargo new --bin loadout-server
WORKDIR /loadout-server

COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY empty .env* ./
COPY build.rs build.rs

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm ./target/release/deps/loadout_server*
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /loadout-server/target/release/loadout-server /usr/src/loadout-server

CMD ["/usr/src/loadout-server"]