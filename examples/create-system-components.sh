#!/usr/bin/env bash
# Create component definitions for the ECS systems
# This defines all components referenced in the system markdown files

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

echo "Creating component definitions for ECS systems..."
echo ""

echo "  Creating Health component..."
$STIGCTL componentdefinition create Health '{
  "type": "object",
  "properties": {
    "current": {"type": "number", "minimum": 0},
    "maximum": {"type": "number", "minimum": 1}
  },
  "required": ["current", "maximum"]
}'

echo "  Creating PoisonEffect component..."
$STIGCTL componentdefinition create PoisonEffect '{
  "type": "object",
  "properties": {
    "damage_per_tick": {"type": "number", "minimum": 0},
    "duration": {"type": "number", "minimum": 0}
  },
  "required": ["damage_per_tick", "duration"]
}'

echo "  Creating BurnEffect component..."
$STIGCTL componentdefinition create BurnEffect '{
  "type": "object",
  "properties": {
    "damage_per_tick": {"type": "number", "minimum": 0},
    "duration": {"type": "number", "minimum": 0}
  },
  "required": ["damage_per_tick", "duration"]
}'

echo "  Creating StatusEffects component..."
$STIGCTL componentdefinition create StatusEffects '{
  "type": "object",
  "properties": {
    "effects": {
      "type": "array",
      "items": {"type": "string"}
    }
  },
  "required": ["effects"]
}'

echo "  Creating Hunger component..."
$STIGCTL componentdefinition create Hunger '{
  "type": "object",
  "properties": {
    "current": {"type": "number", "minimum": 0},
    "maximum": {"type": "number", "minimum": 1}
  },
  "required": ["current", "maximum"]
}'

echo "  Creating Metabolism component..."
$STIGCTL componentdefinition create Metabolism '{
  "type": "object",
  "properties": {
    "hunger_rate": {"type": "number", "minimum": 0}
  },
  "required": ["hunger_rate"]
}'

echo "  Creating Shield component..."
$STIGCTL componentdefinition create Shield '{
  "type": "object",
  "properties": {
    "current": {"type": "number", "minimum": 0},
    "maximum": {"type": "number", "minimum": 0},
    "recharge_rate": {"type": "number", "minimum": 0}
  },
  "required": ["current", "maximum", "recharge_rate"]
}'

echo "  Creating CombatState component..."
$STIGCTL componentdefinition create CombatState '{
  "type": "object",
  "properties": {
    "time_since_damage": {"type": "number", "minimum": 0},
    "in_combat": {"type": "boolean"}
  },
  "required": ["time_since_damage", "in_combat"]
}'

echo "  Creating MagicAnnotation component..."
$STIGCTL componentdefinition create MagicAnnotation '{
  "type": "object",
  "properties": {
    "magic_type": {"type": "string"},
    "power": {"type": "number", "minimum": 0}
  },
  "required": ["magic_type"]
}'

echo "  Creating Healer component..."
$STIGCTL componentdefinition create Healer '{
  "type": "object",
  "properties": {
    "power": {"type": "number", "minimum": 0},
    "aura_radius": {"type": "number", "minimum": 0}
  },
  "required": ["power", "aura_radius"]
}'

echo "  Creating Position component..."
$STIGCTL componentdefinition create Position '{
  "type": "object",
  "properties": {
    "x": {"type": "number"},
    "y": {"type": "number"},
    "z": {"type": "number"}
  },
  "required": ["x", "y"]
}'

echo ""
echo "================================================"
echo "Component Definition Setup Complete!"
echo "================================================"
echo ""
echo "Created component definitions:"
echo "  - Health: health points with current and maximum"
echo "  - PoisonEffect: poison damage over time"
echo "  - BurnEffect: burn damage over time"
echo "  - StatusEffects: array of status effect names"
echo "  - Hunger: hunger levels with current and maximum"
echo "  - Metabolism: metabolic rate for hunger"
echo "  - Shield: energy shield with recharge"
echo "  - CombatState: combat timing information"
echo "  - MagicAnnotation: magical effects and properties"
echo "  - Healer: healing power and aura radius"
echo "  - Position: spatial coordinates (x, y, z)"
echo ""
echo "To view all component definitions:"
echo "  $STIGCTL componentdefinition list"
echo ""
echo "To install systems:"
echo "  ./examples/install-all-systems.sh"
echo ""
