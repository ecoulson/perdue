FROM clux/muslrust:stable AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin perdue

FROM alpine:3.19 AS runtime
WORKDIR /app
RUN apk add sqlite ca-certificates && mkdir database/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/perdue perdue
COPY configuration configuration
COPY data data
COPY assets assets
ENV ENVIRONMENT=production
ENTRYPOINT ["./perdue"]
