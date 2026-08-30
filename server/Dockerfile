# Build the server binary and its browser dashboard. Client capture binaries are
# never included in this image, which makes the server/client boundary explicit.
# Cargo.lock contains dependencies requiring Rust 1.88+. Keep this aligned with
# the repository's stable toolchain rather than the obsolete 1.85 minimum.
FROM rust:1.98-bookworm AS builder
WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY tests ./tests
# sqlx::migrate! embeds these at compile time; without this copy a production
# image cannot build the same schema bootstrap used by the local server binary.
COPY server/migrations ./server/migrations

RUN cargo build --locked --release -p server

FROM node:22-bookworm-slim AS dashboard-builder
WORKDIR /dashboard

COPY apps/desktop-ui/ui/package.json apps/desktop-ui/ui/pnpm-lock.yaml apps/desktop-ui/ui/pnpm-workspace.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY apps/desktop-ui/ui ./
RUN pnpm build

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/trajectory-server /usr/local/bin/trajectory-server
COPY --from=dashboard-builder /dashboard/dist /opt/trajectory/dashboard
ENV DASHBOARD_ASSETS_DIR=/opt/trajectory/dashboard
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/trajectory-server"]
