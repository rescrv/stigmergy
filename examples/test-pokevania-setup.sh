#!/usr/bin/env bash
set -euo pipefail

echo "========================================="
echo "  Pokevania Setup Validation Test"
echo "========================================="
echo ""

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

echo "Step 1: Checking if server is running..."
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo "ERROR: Server not running at $BASE_URL"
    echo "Please start with: cargo run --bin stigmergyd"
    exit 1
fi
echo "✓ Server is running"
echo ""

echo "Step 2: Checking for Pokemon components..."
if ! $STIGCTL componentdefinition list 2>/dev/null | grep -q "PokemonSpecies"; then
    echo "✗ Pokemon components not found"
    echo "Run: cd examples/pokemon && ./setup-pokemon-world.sh"
    POKEMON_MISSING=1
else
    echo "✓ Pokemon components exist"
    POKEMON_MISSING=0
fi
echo ""

echo "Step 3: Checking for Castlevania components..."
if ! $STIGCTL componentdefinition list 2>/dev/null | grep -q "HunterProfile"; then
    echo "✗ Castlevania components not found"
    echo "Run: cd examples/castlevania && ./setup-castlevania-world.sh"
    CASTLEVANIA_MISSING=1
else
    echo "✓ Castlevania components exist"
    CASTLEVANIA_MISSING=0
fi
echo ""

if [ "$POKEMON_MISSING" -eq 1 ] || [ "$CASTLEVANIA_MISSING" -eq 1 ]; then
    echo "========================================="
    echo "Prerequisites not met!"
    echo "========================================="
    echo ""
    echo "To setup Pokevania, you must first:"
    if [ "$POKEMON_MISSING" -eq 1 ]; then
        echo "  1. cd examples/pokemon && ./setup-pokemon-world.sh"
    fi
    if [ "$CASTLEVANIA_MISSING" -eq 1 ]; then
        echo "  2. cd examples/castlevania && ./setup-castlevania-world.sh"
    fi
    echo ""
    echo "Then you can run:"
    echo "  cd examples/pokevania && ./setup-pokevania-world.sh"
    exit 1
fi

echo "========================================="
echo "All prerequisites met!"
echo "========================================="
echo ""
echo "You can now run:"
echo "  cd examples/pokevania"
echo "  ./setup-pokevania-world.sh"
echo "  ./pokevania-story.sh"
echo ""
