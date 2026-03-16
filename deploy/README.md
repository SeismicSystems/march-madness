# Deployment Guide

Production server: `brackets.seismictest.net` running Ubuntu.
Repo lives at `/home/ubuntu/march-madness` on the server.

## Prerequisites

```bash
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev nginx certbot python3-certbot-nginx supervisor redis-server
```

## Redis

Redis is used for persistent chain metadata storage (indexer + server). It's managed by systemd, not supervisor.

```bash
# Install (done above via apt)
# Verify it's running:
sudo systemctl status redis-server

# Enable on boot (should be automatic, but just in case):
sudo systemctl enable redis-server

# Config is at /etc/redis/redis.conf
# Default bind is 127.0.0.1, which is what we want.
# Default port 6379, no auth needed for local-only access.
```

To test:
```bash
redis-cli ping
# Should return PONG
```

## Nginx + SSL

We use nginx to serve the static frontend and reverse-proxy `/api/*` to the Rust server on port 3000.
Certbot handles SSL certificate provisioning and renewal via Let's Encrypt.

### Setup

```bash
# Copy nginx config
sudo cp deploy/nginx.conf /etc/nginx/sites-available/brackets.seismictest.net
sudo ln -sf /etc/nginx/sites-available/brackets.seismictest.net /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default

# Test and reload
sudo nginx -t
sudo systemctl reload nginx

# Provision SSL certificate (will modify nginx.conf to add SSL blocks)
sudo certbot --nginx -d brackets.seismictest.net

# Certbot auto-renewal is installed via systemd timer:
sudo systemctl status certbot.timer
```

After certbot runs, it will add the SSL listen directives and certificate paths to the nginx config automatically. Renewal is automatic via the certbot systemd timer.

## Supervisor

Supervisor manages long-running processes: the API server, the chain indexer, and the NCAA live feed.

### Setup

```bash
# Install (done above via apt)
# Copy config
sudo cp deploy/supervisor.conf /etc/supervisor/conf.d/march-madness.conf

# Create log directory (supervisor usually handles this, but just in case)
sudo mkdir -p /var/log/supervisor

# Reload and start
sudo supervisorctl reread
sudo supervisorctl update
sudo supervisorctl start all
```

### Managed processes

| Process | Binary | Description |
|---------|--------|-------------|
| `mm-server` | `target/release/server` | HTTP API server (port 3000) |
| `mm-indexer` | `target/release/indexer listen` | Chain event listener, writes to Redis |
| `mm-ncaa-feed` | `target/release/ncaa-feed` | NCAA live score poller, writes `status.json` |

### Common commands

```bash
sudo supervisorctl status                  # Check all processes
sudo supervisorctl restart mm-server       # Restart a specific process
sudo supervisorctl tail -f mm-server       # Follow stdout logs
sudo supervisorctl tail -f mm-server stderr  # Follow stderr logs
```

## Building

```bash
cd /home/ubuntu/march-madness

# Build Rust binaries
cargo build --release

# Build frontend
bun install
bun run --filter @march-madness/web build
```

## Environment Variables

Beyond what's in `.env.example`, the following are needed for production:

| Variable | Used by | Description |
|----------|---------|-------------|
| `VITE_RPC_URL` | `mm-indexer` (supervisor) | Seismic RPC endpoint (e.g. `https://rpc.seismictest.net`). The indexer falls back to this env var when `--rpc-url` is not passed. Already in `.env.example`. |
| `REDIS_URL` | `mm-server`, `mm-indexer` | Redis connection string. Defaults to `redis://127.0.0.1:6379` — only set if using non-default. |

Note: `TOURNAMENT_API_KEY` is not needed in this setup. The `ncaa-feed` process writes `status.json` directly to disk, and the server reads it from the same file. The `POST /tournament-status` endpoint exists for remote updates but is unnecessary when both processes run on the same machine.

These are referenced in `deploy/supervisor.conf` via `%(ENV_VAR)s` syntax. Either export them in the shell before running `supervisorctl`, or set them in `/etc/supervisor/supervisord.conf` under `[supervisord]` → `environment=`.

To make env vars available to supervisor globally, add to `/etc/supervisor/supervisord.conf`:

```ini
[supervisord]
environment=VITE_RPC_URL="https://rpc.seismictest.net"
```

Then `sudo supervisorctl reload`.

## Full Deploy Workflow

```bash
cd /home/ubuntu/march-madness
git pull origin main

# Build
cargo build --release
bun install && bun run --filter @march-madness/web build

# Restart services
sudo supervisorctl restart all
```
