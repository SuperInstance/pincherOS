#!/usr/bin/env bash
# Train a deployment operations agent — teach all production reflexes
set -euo pipefail

PINCHER="${PINCHER:-./target/release/pincher}"

if [ ! -f "$PINCHER" ]; then
    echo "Error: pincher binary not found at $PINCHER"
    echo "Build first: cargo build --release"
    exit 1
fi

echo "🦀 Training deployment operations agent..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Health checks
$PINCHER teach --intent "check service health" --action "curl -sf http://localhost:8080/health"
$PINCHER teach --intent "check database connection" --action "pg_isready -h localhost -p 5432"

# Logs
$PINCHER teach --intent "show recent logs" --action "journalctl -u myapp --since '10 minutes ago' --no-pager"
$PINCHER teach --intent "show error logs" --action "journalctl -u myapp -p err --no-pager | tail -50"
$PINCHER teach --intent "search logs for keyword" --action "journalctl -u myapp --no-pager | rg -i"

# Deployment
$PINCHER teach --intent "deploy latest version" --action "cd /opt/myapp && git pull && cargo build --release && systemctl restart myapp"
$PINCHER teach --intent "rollback to previous version" --action "cd /opt/myapp && git checkout HEAD~1 && cargo build --release && systemctl restart myapp"
$PINCHER teach --intent "restart the service" --action "systemctl restart myapp"
$PINCHER teach --intent "stop the service" --action "systemctl stop myapp"
$PINCHER teach --intent "start the service" --action "systemctl start myapp"

# Monitoring
$PINCHER teach --intent "check disk usage" --action "df -h"
$PINCHER teach --intent "check memory usage" --action "free -h"
$PINCHER teach --intent "check cpu load" --action "uptime"
$PINCHER teach --intent "check running processes" --action "ps aux | head -20"
$PINCHER teach --intent "check open ports" --action "ss -tlnp"

# Database
$PINCHER teach --intent "list database tables" --action "psql -h localhost -c '\\dt'"
$PINCHER teach --intent "count rows in users table" --action "psql -h localhost -c 'SELECT count(*) FROM users'"
$PINCHER teach --intent "show active connections" --action "psql -h localhost -c 'SELECT * FROM pg_stat_activity'"

# Backups
$PINCHER teach --intent "create database backup" --action "pg_dump -h localhost myapp > /backup/myapp-$(date +%Y%m%d).sql"
$PINCHER teach --intent "list available backups" --action "ls -lh /backup/"
$PINCHER teach --intent "restore from backup" --action "psql -h localhost myapp < /backup/myapp-latest.sql"

# SSL / Certificates
$PINCHER teach --intent "check certificate expiry" --action "echo | openssl s_client -connect localhost:443 2>/dev/null | openssl x509 -noout -dates"
$PINCHER teach --intent "renew certificates" --action "certbot renew"

# Network
$PINCHER teach --intent "test external connectivity" --action "curl -sf https://httpbin.org/ip"
$PINCHER teach --intent "check dns resolution" --action "dig +short myapp.example.com"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
$PINCHER reflexes
echo ""
echo "✓ Training complete! Next: pincher pack production-agent.nail"
