#!/usr/bin/env bash
# Smart Home — Teach All Reflexes
# Teaches all the smart home reflexes in one go.
#
# Usage: ./teach-reflexes.sh [/path/to/pincher]
#
# Prerequisites:
#   - PincherOS built and available
#   - Homebridge (or compatible API) running at HOMEBRIDGE_URL

set -euo pipefail

# Resolve the pincher binary
if [[ -n "${1:-}" ]]; then
    PINCHER="$1"
elif [[ -x "./target/release/pincher" ]]; then
    PINCHER="./target/release/pincher"
elif command -v pincher &>/dev/null; then
    PINCHER="pincher"
else
    echo "❌ Cannot find pincher binary. Build it first: cargo build --release"
    exit 1
fi

# Homebridge URL (override with env var)
HOMEBRIDGE_URL="${HOMEBRIDGE_URL:-http://homebridge.local:8581}"

# Optional: path to capability manifest
CAPABILITIES_FLAG=""
if [[ -f "./smart-home-capabilities.toml" ]]; then
    CAPABILITIES_FLAG="--capabilities ./smart-home-capabilities.toml"
fi

echo "🦀 Smart Home Reflex Teacher"
echo "   Pincher:  $PINCHER"
echo "   Homebridge: $HOMEBRIDGE_URL"
echo ""

# ── Lights ───────────────────────────────────────────────────────────────
echo "💡 Teaching light reflexes..."

"$PINCHER" teach \
    --intent "turn on the kitchen lights" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/lights/kitchen/on" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "turn off the kitchen lights" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/lights/kitchen/off" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "dim the living room lights" \
    --action "curl -s '${HOMEBRIDGE_URL}/api/lights/living?brightness=30'" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "turn off all lights" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/lights/all/off" \
    $CAPABILITIES_FLAG

# ── Sensors ──────────────────────────────────────────────────────────────
echo "🌡️  Teaching sensor reflexes..."

"$PINCHER" teach \
    --intent "what's the temperature" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/sensors/temp" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "check humidity level" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/sensors/humidity" \
    $CAPABILITIES_FLAG

# ── Thermostat ───────────────────────────────────────────────────────────
echo "🔥 Teaching thermostat reflexes..."

"$PINCHER" teach \
    --intent "set thermostat to 72" \
    --action "curl -s '${HOMEBRIDGE_URL}/api/thermostat?temp=72'" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "set thermostat to 68" \
    --action "curl -s '${HOMEBRIDGE_URL}/api/thermostat?temp=68'" \
    $CAPABILITIES_FLAG

# ── Locks ────────────────────────────────────────────────────────────────
echo "🔒 Teaching lock reflexes..."

"$PINCHER" teach \
    --intent "lock the front door" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/locks/front/lock" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "unlock the front door" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/locks/front/unlock" \
    $CAPABILITIES_FLAG

# ── Cameras ──────────────────────────────────────────────────────────────
echo "📷 Teaching camera reflexes..."

"$PINCHER" teach \
    --intent "show me camera feed" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/cameras/front/snapshot -o /tmp/camera-snapshot.jpg" \
    $CAPABILITIES_FLAG

"$PINCHER" teach \
    --intent "show me the back yard camera" \
    --action "curl -s ${HOMEBRIDGE_URL}/api/cameras/back/snapshot -o /tmp/camera-back-snapshot.jpg" \
    $CAPABILITIES_FLAG

# ── Verify ───────────────────────────────────────────────────────────────
echo ""
echo "✅ All reflexes taught! Here's what your crab knows:"
echo ""
"$PINCHER" reflexes
echo ""
echo "Try: pincher do \"turn on kitchen lights\""
