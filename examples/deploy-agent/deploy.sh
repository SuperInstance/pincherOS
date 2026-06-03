#!/usr/bin/env bash
# Deploy a PincherOS agent to a remote server
# Usage: ./deploy.sh user@host [nail-file] [remote-path]
set -euo pipefail

HOST="${1:?Usage: deploy.sh user@host [nail-file] [remote-path]}"
NAIL_FILE="${2:-production-agent.nail}"
REMOTE_PATH="${3:-~/}"

if [ ! -f "$NAIL_FILE" ]; then
    echo "Error: .nail file not found: $NAIL_FILE"
    echo "Create one first: pincher pack production-agent.nail"
    exit 1
fi

echo "🦀 Deploying agent to $HOST"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 1: Transfer the .nail file
echo ""
echo "⏳ Transferring $NAIL_FILE to $HOST:$REMOTE_PATH..."
scp "$NAIL_FILE" "$HOST:$REMOTE_PATH"
echo "✓ Transfer complete"

# Step 2: Transfer the pincher binary (if not already on target)
echo ""
echo "⏳ Checking if pincher is installed on target..."
if ssh "$HOST" "test -f ~/pincher && echo 'EXISTS'" 2>/dev/null | grep -q EXISTS; then
    echo "✓ pincher binary already on target"
else
    echo "⏳ Transferring pincher binary..."
    scp ./target/release/pincher "$HOST:~/pincher"
    ssh "$HOST" "chmod +x ~/pincher"
    echo "✓ Binary transferred"
fi

# Step 3: Unpack on the target
echo ""
echo "⏳ Unpacking agent on target..."
ssh "$HOST" "~/pincher unpack $REMOTE_PATH$(basename $NAIL_FILE)"
echo "✓ Agent unpacked"

# Step 4: Verify
echo ""
echo "⏳ Verifying agent status on target..."
ssh "$HOST" "~/pincher status"

echo ""
echo "✓ Deployment complete!"
echo ""
echo "To start the RPC server on the target:"
echo "  ssh $HOST '~/pincher rpc --port 9876 &'"
