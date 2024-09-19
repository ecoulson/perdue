FROM clux/muslrust:stable AS chef
RUN cargo install cargo-chef
WORKDIR /tools

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /tools/recipe.json recipe.json
COPY . .
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
RUN cargo build --release --target x86_64-unknown-linux-musl --bin migrate

FROM alpine:3.19 AS runtime
COPY --from=builder /tools/target/x86_64-unknown-linux-musl/release/migrate /usr/local/bin/migrate
ENV ENVIRONMENT=production
