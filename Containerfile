# =============================================================================
# snake — Alpine container image (Unraid-compatible)
# =============================================================================
# Build stage: rust:alpine (musl libc).
# Runtime stage: alpine (musl libc) — same userspace as the build, so the
# dynamically-linked backend binary runs natively.
#
# Unraid notes:
#   - Runs as UID 99:GID 100 (nobody:nogroup) so host-mounted `/config`
#     volumes are writable on Unraid.
#   - Data directory is `/config` (Unraid convention). Override with
#     SNAKE_DATA_DIR env var if needed.
#   - Listens on 0.0.0.0:<port> for bridge networking.
#
# Build (Podman):
#   cd ~/projects/studio2201/snake
#   podman build -f Containerfile.ubi -t snake:latest .
#
# Run (Unraid-style):
#   podman run -d --name snake \
#     -p <host-port>:4501 \
#     -v /mnt/user/appdata/snake:/config \
#     -e SNAKE_PIN=<your-pin> \
#     snake:latest
#

# Compare size:
#   podman images | grep -E 'snake|studio2201/snake'
# =============================================================================

# ---------- build: Alpine + musl ----------
FROM docker.io/library/rust:alpine AS builder

# Build-time deps. git fetches the shared-assets git dep; musl-dev/pkgconf
# support any crates that need C compilation.
RUN apk add --no-cache \
        bash \
        ca-certificates \
        curl \
        git \
        musl-dev \
        pkgconf

WORKDIR /src
COPY rust-toolchain.toml ./

# Install the toolchain pinned in rust-toolchain.toml + the wasm32 target.
RUN rustup show \
 && rustup target add wasm32-unknown-unknown

# Install trunk and wasm-bindgen-cli from prebuilt musl binaries (fast).
ARG TRUNK_VERSION=0.21.3
RUN curl -sSfL -o /tmp/trunk.tar.gz \
        "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-musl.tar.gz" \
 && tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin/ \
 && chmod +x /usr/local/bin/trunk \
 && rm /tmp/trunk.tar.gz

ARG WASM_BINDGEN_VERSION=0.2.121
RUN curl -sSfL -o /tmp/wb.tar.gz \
        "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
 && tar -xzf /tmp/wb.tar.gz -C /tmp/ \
 && mv /tmp/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl/wasm-bindgen /usr/local/bin/wasm-bindgen \
 && chmod +x /usr/local/bin/wasm-bindgen \
 && rm -rf /tmp/wb.tar.gz /tmp/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl
# Cache deps: copy manifests first.
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml

# Dummy sources so the dependency layer can compile without full sources.
RUN mkdir -p backend/src frontend/src \
 && echo 'fn main() {}' > backend/src/main.rs \
 && echo 'fn main() {}' > frontend/src/main.rs

# Fetch crates (needs network for crates.io + shared-assets git tag).
RUN cargo fetch

# Real sources (includes assets/shared-assets styles for trunk).
COPY assets assets
COPY backend backend
COPY frontend frontend

WORKDIR /src/frontend
ENV CARGO_TARGET_DIR=/src/target
RUN trunk build --release

WORKDIR /src
RUN cargo build --release -p backend \
 && strip /src/target/release/backend

# ---------- runtime: Alpine ----------
FROM docker.io/library/alpine:3.20

LABEL org.opencontainers.image.title="snake" \
      org.opencontainers.image.description="snake (Alpine, built on Alpine; Unraid-compatible)" \
      org.opencontainers.image.source="https://github.com/studio2201/snake" \
      org.opencontainers.image.base.name="docker.io/library/alpine:3.20"

# Runtime deps. tini handles PID 1 and signal forwarding. wget for HEALTHCHECK.
# ca-certificates for outbound HTTPS.
#
# The container runs as UID 99:GID 100 (nobody:nogroup on Unraid) so that
# `/config` can be mounted from the host and written to without perms
# surprises. We don't create an /etc/passwd user entry — instead we chown
# everything to 99:100 and `USER 99:100` runs the binary directly.
RUN apk add --no-cache \
        ca-certificates \
        tini \
        wget \
 && mkdir -p /config

COPY --from=builder /src/target/release/backend /app/backend
COPY --from=builder /src/target/release/sh /usr/local/bin/snake
RUN chmod +x /usr/local/bin/snake
COPY --from=builder /src/frontend/dist /app/frontend/dist

# Runtime UID is 99 (nobody:nogroup). Some apps generate PWA manifests or
# asset listings into /app/frontend/dist at boot (pad, trace, pulse). chown
# /app (binary + frontend bundle) and /config (data dir) to 99:100, and
# make the frontend bundle writable so the runtime generator can write
# into /app/frontend/dist/assets/.
RUN chown -R 99:100 /app /config \
 && chmod -R u+w /app/frontend/dist

# Rustle's source code references 'dist/' (without the 'frontend/'
# prefix). Symlink so the path resolves either way.
RUN mkdir -p /app/data \
 && rm -rf /app/data \
 && ln -s /config /app/data \
 && ln -sfn /app/frontend/dist /app/dist

WORKDIR /app
USER 99:100

ENV PORT=4501 \
    RUST_LOG=info \
    SNAKE_DATA_DIR=/config


EXPOSE 4501

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -qO- "http://127.0.0.1:${PORT:-4501}/health" >/dev/null || exit 1

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/app/backend"]
