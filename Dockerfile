# Stage 1: Build frontend
FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
FROM rust:1.82-bookworm AS backend
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --bin loom_cli

# Stage 3: Final runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates git && rm -rf /var/lib/apt/lists/*

COPY --from=backend /build/target/release/loom_cli /usr/local/bin/loom
COPY --from=frontend /build/frontend/dist /opt/loom/frontend/dist

WORKDIR /opt/loom

ENV DATABASE_URL=postgres://loom:loom@postgres/mysite_localhost

EXPOSE 8000

CMD ["loom", "serve", "--frontend-dir", "/opt/loom/frontend/dist"]
