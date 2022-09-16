FROM --platform=$BUILDPLATFORM rust:latest AS build
ARG TARGETPLATFORM

RUN apt-get update && \
    case "$TARGETPLATFORM" in \
    "linux/arm/v7") echo armv7-unknown-linux-musleabihf > /rust_target.txt && apt-get install -y binutils-arm-linux-gnueabihf ;; \
    "linux/arm/v6") echo arm-unknown-linux-musleabi > /rust_target.txt && apt-get install -y binutils-arm-linux-gnueabihf ;; \
    "linux/aarch64") echo aarch64-unknown-linux-gnu > /rust_target.txt ;; \
    "linux/amd64") echo x86_64-unknown-linux-gnu > /rust_target.txt ;; \
    *) exit 1 ;; \
    esac

# RUN apt-get update && apt-get -y install binutils-arm-linux-gnueabihf gcc-arm-linux-gnueabihf musl-tools && \
#     ln -s /usr/bin/arm-linux-gnueabihf-gcc /usr/bin/arm-linux-musleabihf-gcc
# RUN apt-get update && apt-get -y install binutils-arm-linux-gnueabihf
WORKDIR /app

RUN rustup target add $(cat /rust_target.txt)

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --target $(cat /rust_target.txt)
RUN cp target/$(cat /rust_target.txt)/release/loadout-server .
RUN ls .

FROM alpine:latest
ENV \
    RUST_BACKTRACE=full

COPY --from=build /app/loadout-server ./
RUN ls .

ENTRYPOINT ["./loadout-server"]