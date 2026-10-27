# syntax=docker/dockerfile:1

# --- Build Stage ---
FROM rust:1.99.1-alpine AS builder

# musl-dev is required for the static-libc link that lets the binary run on a
# scratch-like runtime.
RUN apk add --no-cache musl-dev

WORKDIR /app

# Compile dependencies against a stub crate first so this layer is only
# invalidated when the manifests change, not on every source edit.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && echo 'fn main() {}' > src/bin/vectify.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked --bin vectify \
    && rm -rf src

COPY src ./src

# Touch the real sources so cargo does not reuse the stub build above.
RUN touch src/lib.rs src/bin/vectify.rs \
    && cargo build --release --locked --bin vectify

# --- Final Stage ---
FROM alpine:3.24

# wget (busybox) backs the health check; the tracer itself needs nothing else.
RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 vectify \
    && adduser -S -u 10001 -G vectify -H vectify

COPY --from=builder /app/target/release/vectify /usr/local/bin/vectify

USER vectify
WORKDIR /srv

EXPOSE 8080

ENV PORT=8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -q -O /dev/null "http://127.0.0.1:${PORT}/health" || exit 1

ENTRYPOINT ["vectify"]
CMD ["serve", "--bind", "0.0.0.0:8080"]
