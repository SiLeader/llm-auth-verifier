FROM rust:1.98.1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /work

COPY . .

RUN cargo build --release && \
    cp target/release/llm-auth-verifier /llm-auth-verifier

FROM scratch

COPY --from=builder /llm-auth-verifier /llm-auth-verifier

CMD ["/llm-auth-verifier"]
