FROM --platform=$BUILDPLATFORM rust:latest AS build
ARG TARGETPLATFORM

RUN case "$TARGETPLATFORM" in \
        "linux/arm/v7") echo armv7-unknown-linux-musleabihf > /rust_target.txt ;; \
        "linux/arm/v6") echo arm-unknown-linux-musleabi > /rust_target.txt ;;\
        *) exit 1 ;; \
    esac
RUN rustup target add $(cat /rust_target.txt)
RUN apt-get update && apt-get -y install binutils-arm-linux-gnueabihf
WORKDIR /app

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock empty .env* build.rs ./
COPY src ./src

RUN cargo build --release --target $(cat /rust_target.txt)
RUN cp target/$(cat rust_target.txt)/release/loadout-server .

FROM apline:latest
ENV \
    RUST_BACKTRACE=full
WORKDIR /app
COPY --from=build /app/loadout-server ./

ENTRYPOINT ["./loadout-server"]