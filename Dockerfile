FROM rust:slim-bullseye AS builder

WORKDIR /app
COPY . .
RUN apt update && apt install -y clang libclang-dev libssl-dev libopencv-dev && cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/yakudobot_rs /usr/local/bin/yakudobot_rs
RUN apt update && apt install -y libssl1.1 libopencv-core4.5 libopencv-imgcodecs4.5 libopencv-imgproc4.5 && rm -rf /var/lib/apt/lists/*

CMD ["/usr/local/bin/yakudobot_rs"]