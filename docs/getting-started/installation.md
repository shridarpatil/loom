# Installation

## Prerequisites

- **Rust** 1.75+ (only for building the framework itself)
- **PostgreSQL** 14+
- **Node.js** 18+ (for building the frontend)

## Build from Source

```bash
git clone https://github.com/anthropics/loom.git
cd loom

# Build the CLI
cargo build --release

# Build the frontend
cd frontend && npm install && npx vite build && cd ..

# The binary is at target/release/loom_cli
alias loom="./target/release/loom_cli"
```

## Create Your First Site

```bash
# Create a PostgreSQL database
createdb loom_dev

# Initialize the site
loom new-site mysite.localhost --db-url postgres://localhost/loom_dev

# Start the server
loom serve --db-url postgres://localhost/loom_dev

# Open http://localhost:8000
# Login: Administrator / admin
```

## Docker

```bash
docker-compose up -d
```

This starts PostgreSQL and the Loom server. The Desk UI is available at `http://localhost:8000`.
