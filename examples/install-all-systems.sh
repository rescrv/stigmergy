#!/usr/bin/env bash
# Install all system definitions from markdown files
# This script creates systems from all .md files in the examples directory

set -euo pipefail

# Build stigctl first
echo "Building stigctl..."
cargo build --bin stigctl
echo ""

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

echo "Installing systems from markdown files..."
echo ""

# Find all system markdown files in examples directory
# Exclude README files and files in subdirectories
SYSTEM_FILES=(
    "examples/damage-over-time-system.md"
    "examples/healing-aura-system.md"
    "examples/hunger-system.md"
    "examples/resurrection-system.md"
    "examples/shield-regeneration-system.md"
)

SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_SYSTEMS=()

for system_file in "${SYSTEM_FILES[@]}"; do
    if [ ! -f "$system_file" ]; then
        echo "  [SKIP] $system_file (not found)"
        continue
    fi

    system_name=$(basename "$system_file" .md)
    echo "  Installing $system_name..."

    if $STIGCTL system create-from-md "$system_file" 2>&1; then
        echo "    [OK] $system_name installed successfully"
        ((SUCCESS_COUNT++))
    else
        echo "    [FAILED] $system_name installation failed"
        ((FAIL_COUNT++))
        FAILED_SYSTEMS+=("$system_name")
    fi
    echo ""
done

echo "================================================"
echo "System Installation Complete!"
echo "================================================"
echo ""
echo "Summary:"
echo "  Successful: $SUCCESS_COUNT"
echo "  Failed:     $FAIL_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
    echo ""
    echo "Failed systems:"
    for failed in "${FAILED_SYSTEMS[@]}"; do
        echo "  - $failed"
    done
fi

echo ""
echo "To view installed systems:"
echo "  $STIGCTL system list"
echo ""

if [ $FAIL_COUNT -gt 0 ]; then
    exit 1
fi
