#!/usr/bin/env bash
# Code Review Pipeline — Run a full review cycle
# Runs all code review reflexes in sequence and reports results.
#
# Usage: ./review-pipeline.sh [/path/to/project] [/path/to/pincher]
#
# If no project path is given, uses the current directory.
# If no pincher path is given, tries ./target/release/pincher, then PATH.

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────
PROJECT_DIR="${1:-.}"
shift 2>/dev/null || true

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

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
PASS=0
FAIL=0
WARN=0
RESULTS=()

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  PincherOS Code Review Pipeline${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  Project: $(cd "$PROJECT_DIR" && pwd)"
echo "  Pincher: $PINCHER"
echo ""

# Change to project directory
cd "$PROJECT_DIR"

# ── Helper: Run a single reflex check ────────────────────────────────────
run_check() {
    local intent="$1"
    local severity="$2"  # "critical", "warning", or "info"

    echo -e "${YELLOW}▶ Running: ${intent}${NC}"

    # Run the reflex and capture output + exit code
    local output
    local exit_code=0
    output=$("$PINCHER" do "$intent" 2>&1) || exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        # Check if the output contains findings (rg returns 0 even with matches)
        if echo "$output" | rg -q "^[0-9]+:" 2>/dev/null; then
            # ripgrep found matches — this is a finding
            local count
            count=$(echo "$output" | rg "^[0-9]+:" | wc -l)
            if [[ "$severity" == "critical" ]]; then
                RESULTS+=("FAIL|$intent|$count findings")
                FAIL=$((FAIL + 1))
                echo -e "  ${RED}✗ FAILED: $count findings${NC}"
            else
                RESULTS+=("WARN|$intent|$count findings")
                WARN=$((WARN + 1))
                echo -e "  ${YELLOW}⚠ WARNING: $count findings${NC}"
            fi
            # Show first 5 findings
            echo "$output" | rg "^[0-9]+:" | head -5 | while read -r line; do
                echo -e "    ${RED}$line${NC}"
            done
        else
            RESULTS+=("PASS|$intent|no findings")
            PASS=$((PASS + 1))
            echo -e "  ${GREEN}✓ PASSED: no findings${NC}"
        fi
    else
        RESULTS+=("FAIL|$intent|exit code $exit_code")
        FAIL=$((FAIL + 1))
        echo -e "  ${RED}✗ FAILED: exit code $exit_code${NC}"
    fi
    echo ""
}

# ── Security Checks ──────────────────────────────────────────────────────
echo -e "${CYAN}── Security Checks ──────────────────────────────────${NC}"
echo ""

run_check "check for sql injection" "critical"
run_check "check for hardcoded secrets" "critical"
run_check "check for unsafe deserialization" "critical"

# ── Quality Checks ───────────────────────────────────────────────────────
echo -e "${CYAN}── Quality Checks ───────────────────────────────────${NC}"
echo ""

run_check "find todo comments" "warning"
run_check "check error handling" "warning"
run_check "check for missing docs" "info"

# ── Convention Checks ────────────────────────────────────────────────────
echo -e "${CYAN}── Convention Checks ────────────────────────────────${NC}"
echo ""

run_check "enforce naming conventions" "warning"

# ── Test Checks ──────────────────────────────────────────────────────────
echo -e "${CYAN}── Test Checks ──────────────────────────────────────${NC}"
echo ""

run_check "check test coverage" "warning"
run_check "check for missing tests" "warning"

# ── Summary ──────────────────────────────────────────────────────────────
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Review Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

for result in "${RESULTS[@]}"; do
    IFS='|' read -r status intent detail <<< "$result"
    case "$status" in
        PASS)  echo -e "  ${GREEN}✓${NC} $intent — $detail" ;;
        WARN)  echo -e "  ${YELLOW}⚠${NC} $intent — $detail" ;;
        FAIL)  echo -e "  ${RED}✗${NC} $intent — $detail" ;;
    esac
done

echo ""
echo -e "  ${GREEN}Passed:   $PASS${NC}"
echo -e "  ${YELLOW}Warnings: $WARN${NC}"
echo -e "  ${RED}Failed:   $FAIL${NC}"
echo ""

if [[ $FAIL -gt 0 ]]; then
    echo -e "${RED}❌ Review failed. Fix critical issues before merging.${NC}"
    exit 1
elif [[ $WARN -gt 0 ]]; then
    echo -e "${YELLOW}⚠️  Review passed with warnings. Consider addressing them.${NC}"
    exit 0
else
    echo -e "${GREEN}✅ Review passed. All checks clean.${NC}"
    exit 0
fi
