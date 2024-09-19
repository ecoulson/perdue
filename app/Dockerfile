FROM clux/muslrust:stable AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
COPY . .
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
RUN cargo build --release --target x86_64-unknown-linux-musl --bin perdue

FROM ecoulson/union-base:latest AS runtime
WORKDIR /app
RUN apk add sqlite ca-certificates && mkdir database/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/perdue perdue
COPY migrations migrations
COPY configuration configuration
COPY data data
COPY assets assets
ENV ENVIRONMENT=production
ENTRYPOINT ["./perdue"]
