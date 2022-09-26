FROM rust:slim-bullseye AS builder

WORKDIR /app
COPY . .
RUN apt update && apt install -y libopencv-dev clang && cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/yakudobot_rs /usr/local/bin/yakudobot_rs
RUN apt update && apt install -y libopencv-core4.5 libopencv-imgcodecs4.5 libopencv-imgproc4.5 && rm -rf /var/lib/apt/lists/*

CMD ["/usr/local/bin/yakudobot_rs"]