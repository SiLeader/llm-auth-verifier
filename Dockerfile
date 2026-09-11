FROM rust:1.98.1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /work

COPY . .

RUN cargo build --release && \
    cp target/release/llm-auth-verifier /llm-auth-verifier

FROM scratch

COPY --from=builder /llm-auth-verifier /llm-auth-verifier

USER 1000:1000

CMD ["/llm-auth-verifier", "--listen", "0.0.0.0:9731"]
