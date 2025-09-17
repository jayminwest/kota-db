# KotaDB Docker Production Container

This document describes how to use the production-ready Docker container for KotaDB.

## Quick Start

### Using Docker directly

```bash
# Create data directory with proper permissions
mkdir -p ./kotadb-data
chmod 777 ./kotadb-data

# Run KotaDB server
docker run -d \
  --name kotadb \
  -p 8080:8080 \
  -v ./kotadb-data:/data \
  kotadb:latest
```

### Using Docker Compose

```bash
# Start production setup (API + MCP servers)
docker compose -f docker-compose.prod.yml up -d

# Stop the service
docker compose -f docker-compose.prod.yml down


### Streamable MCP server only

```bash
# Launch just the MCP endpoint (Streamable HTTP on port 8484)
docker compose -f docker-compose.prod.yml up -d kotadb-mcp

# Tail MCP logs
docker compose -f docker-compose.prod.yml logs -f kotadb-mcp
```
```

## Container Features

- **Multi-stage build** for a compact runtime layer (no ONNX runtime)
- **Non-root user** for security (kotadb:kotadb, uid:gid 1001:1001)
- **Bundled binaries** for CLI server, SaaS API server, and MCP HTTP server
- **Data persistence** via Docker volumes or bind mounts
- **Ready for docker-compose.prod.yml** (runs API + MCP, optional quickstart demos)

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `KOTADB_PORT` | `8080` | Port for HTTP server |
| `KOTADB_DATA_DIR` | `/data` | Database data directory |
| `KOTADB_LOG_LEVEL` | `info` | Log level (debug, info, warn, error) |
| `RUST_LOG` | `info` | Rust logging configuration |
| `RUST_BACKTRACE` | `0` | Enable backtraces (0=off, 1=on) |

## Usage Examples

### Basic usage with custom port

```bash
docker run -d \
  --name kotadb \
  -p 3000:3000 \
  -e KOTADB_PORT=3000 \
  -v ./my-data:/data \
  kotadb:latest
```

### Development with debug logging

```bash
docker run -d \
  --name kotadb-debug \
  -p 8080:8080 \
  -e RUST_LOG=debug \
  -e RUST_BACKTRACE=1 \
  -v ./kotadb-data:/data \
  kotadb:latest
```

## API Endpoints

⚠️ **Updated**: Document CRUD endpoints have been removed. The following endpoints are available:

- `GET /health` - Health check
- `GET /stats` - System statistics
- `POST /validate/*` - Validation endpoints

**Migration**: For document operations, use the codebase intelligence API via MCP server or client libraries instead.

## Health Checks

`docker-compose.prod.yml` defines health checks for both the HTTP API (`/health`) and the MCP server (Streamable HTTP `initialize`). If you run the container directly you can probe those endpoints manually:

```bash
# API server health
curl -fsS http://localhost:8080/health

# MCP server health (Streamable HTTP handshake)
curl -fsS -X POST http://localhost:8484/mcp \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2025-06-18' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

## Data Persistence

Data is persisted in the `/data` directory inside the container. Mount a volume to ensure data survives container restarts:

```bash
# Named volume
docker run -v kotadb-data:/data kotadb:latest

# Host directory (requires proper permissions)
docker run -v $(pwd)/kotadb-data:/data kotadb:latest
```

**Important**: When using host directories, ensure they have write permissions for uid 1001 (the kotadb user inside the container).

## Building from Source

```bash
# Build production image
docker build -f Dockerfile.prod -t kotadb:latest .

# Build and start with compose
docker-compose -f docker-compose.prod.yml up --build
```

## Troubleshooting

### Permission Denied Errors

If you see "Permission denied" errors, ensure the mounted directory has proper permissions:

```bash
chmod 777 ./kotadb-data
```

### Health Check Failures

If health checks fail, check the logs:

```bash
docker logs kotadb
```

Common issues:
- Port conflicts (change KOTADB_PORT)
- Data directory permissions
- Insufficient system resources

### Container Won't Start

1. Check Docker logs: `docker logs kotadb`
2. Verify image was built correctly: `docker images kotadb`
3. Check port availability: `netstat -an | grep 8080`
4. Ensure data directory exists and is writable

## Production Deployment

For production use:

1. Use a reverse proxy (nginx/traefik) for SSL termination
2. Set up proper monitoring and log aggregation
3. Use Docker secrets for sensitive configuration
4. Implement backup strategies for the data volume
5. Configure resource limits:

```bash
docker run -d \
  --name kotadb \
  --memory=512m \
  --cpus="0.5" \
  -p 8080:8080 \
  -v ./kotadb-data:/data \
  kotadb:latest
```

## Security Notes

- Container runs as non-root user (kotadb:1001)
- Binary is stripped of debug symbols
- Uses minimal Alpine base image
- No unnecessary packages installed
- Health checks validate service availability
