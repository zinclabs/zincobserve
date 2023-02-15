FROM public.ecr.aws/docker/library/node:18.12.1-bullseye as webBuilder
WORKDIR /web
COPY ./web /web/

RUN npm install
RUN NODE_OPTIONS="--max-old-space-size=8192" npm run build

FROM public.ecr.aws/docker/library/rust:bullseye as builder
RUN apt-get update
RUN apt-get install -y protobuf-compiler
RUN apt-get install -y cmake curl

RUN rustup toolchain install nightly-2023-01-26
RUN rustup default nightly-2023-01-26
RUN rustup target add x86_64-unknown-linux-gnu
# RUN diff -u <(rustc --print cfg) <(rustc -C target-cpu=native --print cfg)
RUN rustc --print cfg
RUN rustc -C target-cpu=native --print cfg

WORKDIR /app
COPY . /app
COPY --from=webBuilder /web/dist web/dist

# RUN cargo build --release
RUN RUSTFLAGS='-C target-feature=+aes,+avx,+avx2,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2' cargo build --release --features tmpcache,zo_functions --target x86_64-unknown-linux-gnu
RUN mv /app/target/x86_64-unknown-linux-gnu/release/zincobserve /app/target/release/zincobserve

# FROM gcr.io/distroless/cc as runtime
FROM public.ecr.aws/debian/debian:bullseye-slim as runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates
RUN apt-get install -y curl htop iftop sysstat procps lsof net-tools
RUN update-ca-certificates
COPY --from=builder /app/target/release/zincobserve /
CMD ["/zincobserve"]
