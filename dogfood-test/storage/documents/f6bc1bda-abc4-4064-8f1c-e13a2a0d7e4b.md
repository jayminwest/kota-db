---
tags:
- file
- kota-db
- ext_yml
---
version: '3.8'

services:
  kotadb:
    build:
      context: .
      dockerfile: Dockerfile.prod
    container_name: kotadb-production
    restart: unless-stopped
    ports:
      - "${KOTADB_PORT:-8080}:8080"
    environment:
      - KOTADB_PORT=8080
      - KOTADB_DATA_DIR=/data
      - KOTADB_LOG_LEVEL=${KOTADB_LOG_LEVEL:-info}
      - RUST_LOG=${RUST_LOG:-info}
      - RUST_BACKTRACE=${RUST_BACKTRACE:-0}
    volumes:
      # Data persistence as specified in issue requirements
      - ./kotadb-data:/data
    command: ["serve", "--port", "8080"]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    networks:
      - kotadb-prod

networks:
  kotadb-prod:
    driver: bridge
