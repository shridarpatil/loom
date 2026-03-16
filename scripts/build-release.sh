#!/usr/bin/env bash
set -euo pipefail

echo "==> Building frontend..."
cd frontend
npm ci --silent
npm run build
cd ..

echo "==> Building Rust binary (release)..."
cargo build --release

echo "==> Packaging..."
DIST="dist/loom"
rm -rf "$DIST"
mkdir -p "$DIST/frontend" "$DIST/apps" "$DIST/sites"

cp target/release/loom_cli "$DIST/loom"
cp -r frontend/dist "$DIST/frontend/dist"

# Copy existing apps if any
if [ -d "apps" ] && [ "$(ls -A apps 2>/dev/null)" ]; then
  cp -r apps/* "$DIST/apps/"
fi

echo ""
echo "Done! Package is in dist/loom/"
echo ""
echo "Deploy:"
echo "  scp -r dist/loom/ user@server:/opt/loom/"
echo ""
echo "On the server:"
echo "  cd /opt/loom"
echo "  DATABASE_URL=postgres://user:pass@localhost/db ./loom serve"
echo ""
echo "Install a 3rd-party app:"
echo "  cd /opt/loom"
echo "  ./loom get-app https://github.com/someone/loom_hr"
echo "  DATABASE_URL=postgres://... ./loom install-app loom_hr"
echo "  # restart the server"
