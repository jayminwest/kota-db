#!/bin/sh
set -e

echo "=== KotaDB API Server Starting ==="
echo "PORT: ${PORT:-8080}"
echo "DATABASE_URL: ${DATABASE_URL:+SET}"
echo "KOTADB_DATA_DIR: ${KOTADB_DATA_DIR:-/data}"
echo "RUST_LOG: ${RUST_LOG:-info}"

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL is not set!"
    exit 1
fi

echo "DATABASE_URL is configured"

# Verify binary exists and is executable
if [ ! -x "/usr/local/bin/kotadb-api-server" ]; then
    echo "ERROR: kotadb-api-server binary not found or not executable!"
    ls -la /usr/local/bin/
    exit 1
fi

echo "Binary check passed"

# Create data directory
mkdir -p "${KOTADB_DATA_DIR:-/data}" || true

# Test database connection first
echo "Testing database connectivity..."

# Start the server with explicit error handling
echo "Starting server on port ${PORT:-8080}..."
echo "Command: kotadb-api-server"
echo "Working directory: $(pwd)"
echo "User: $(whoami)"

# Execute the actual server
echo "Starting kotadb-api-server..."
exec /usr/local/bin/kotadb-api-server 2>&1