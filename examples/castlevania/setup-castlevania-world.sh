#!/usr/bin/env bash
# Setup script for a Castlevania-inspired world using Stigmergy

set -euo pipefail

# Build stigctl first to avoid repeated compilation noise
echo "Building stigctl..."
cargo build --bin stigctl
echo ""

STIGCTL="${STIGCTL:-target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

echo "Creating component definitions..."
# Allow attempting to create definitions even if they already exist
set +e

echo "  Creating HunterProfile component..."
$STIGCTL componentdefinition create HunterProfile '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "order": {"oneOf": [{"type": "string"}, {"type": "null"}]},
    "title": {"type": "string"},
    "renown": {"type": "integer", "minimum": 0},
    "vows": {
      "type": "array",
      "items": {"type": "string"}
    }
  },
  "required": ["name", "title", "renown", "vows"]
}'

echo "  Creating HunterStats component..."
$STIGCTL componentdefinition create HunterStats '{
  "type": "object",
  "properties": {
    "level": {"type": "integer", "minimum": 1, "maximum": 50},
    "current_hp": {"type": "integer"},
    "max_hp": {"type": "integer"},
    "vitality": {"type": "integer"},
    "strength": {"type": "integer"},
    "focus": {"type": "integer"},
    "resilience": {"type": "integer"},
    "speed": {"type": "integer"},
    "resolve": {"type": "integer"},
    "status": {"type": "string", "enum": ["ready", "wounded", "exhausted", "banished"]},
    "experience": {"type": "integer"},
    "experience_to_next_rank": {"type": "integer"}
  },
  "required": ["level", "current_hp", "max_hp", "vitality", "strength", "focus", "resilience", "speed", "resolve", "status", "experience", "experience_to_next_rank"]
}'

echo "  Creating MonsterProfile component..."
$STIGCTL componentdefinition create MonsterProfile '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "species": {"type": "string"},
    "threat_level": {"type": "string", "enum": ["lesser", "greater", "nightmare"]},
    "origin": {"type": "string"},
    "weaknesses": {
      "type": "array",
      "items": {"type": "string"}
    },
    "lair": {"type": "string"},
    "description": {"type": "string"}
  },
  "required": ["name", "species", "threat_level", "weaknesses", "lair"]
}'

echo "  Creating MonsterState component..."
$STIGCTL componentdefinition create MonsterState '{
  "type": "object",
  "properties": {
    "current_hp": {"type": "integer"},
    "max_hp": {"type": "integer"},
    "status": {"type": "string", "enum": ["dormant", "lurking", "rampaging", "banished"]},
    "aggression": {"type": "integer", "minimum": 0, "maximum": 100},
    "enrage": {"type": "integer", "minimum": 0, "maximum": 100}
  },
  "required": ["current_hp", "max_hp", "status", "aggression", "enrage"]
}'

echo "  Creating Relic component..."
$STIGCTL componentdefinition create Relic '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "category": {"type": "string", "enum": ["weapon", "tool", "blessing", "artifact"]},
    "description": {"type": "string"},
    "power": {"type": "string"}
  },
  "required": ["name", "category", "description"]
}'

echo "  Creating Arsenal component..."
$STIGCTL componentdefinition create Arsenal '{
  "type": "object",
  "properties": {
    "primary_weapon": {"type": "string"},
    "backup_weapons": {
      "type": "array",
      "items": {"type": "string"}
    },
    "active_relics": {
      "type": "array",
      "items": {"type": "string"}
    }
  },
  "required": ["primary_weapon", "backup_weapons", "active_relics"]
}'

echo "  Creating Inventory component..."
$STIGCTL componentdefinition create Inventory '{
  "type": "object",
  "properties": {
    "items": {
      "type": "object",
      "additionalProperties": {"type": "integer"}
    }
  },
  "required": ["items"]
}'

echo "  Creating Location component..."
$STIGCTL componentdefinition create Location '{
  "type": "object",
  "properties": {
    "region": {"type": "string"},
    "area": {"type": "string"},
    "x": {"type": "number"},
    "y": {"type": "number"},
    "altitude": {"type": "number"}
  },
  "required": ["region", "area", "x", "y", "altitude"]
}'

echo "  Creating QuestLog component..."
$STIGCTL componentdefinition create QuestLog '{
  "type": "object",
  "properties": {
    "active": {
      "type": "array",
      "items": {"type": "string"}
    },
    "completed": {
      "type": "array",
      "items": {"type": "string"}
    }
  },
  "required": ["active", "completed"]
}'

echo "  Creating Mentor component..."
$STIGCTL componentdefinition create Mentor '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "role": {"type": "string"},
    "dialogue": {
      "type": "array",
      "items": {"type": "string"}
    },
    "offers_training": {"type": "boolean"},
    "location_hint": {"oneOf": [{"type": "string"}, {"type": "null"}]}
  },
  "required": ["name", "role", "dialogue", "offers_training"]
}'

set -e

echo ""
echo "Creating entities and initial world state..."

echo "  Creating hunter entity..."
HUNTER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Hunter: $HUNTER"

echo "  Creating mentor entity..."
MENTOR=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Mentor: $MENTOR"

echo "  Creating monster entity..."
MONSTER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Monster: $MONSTER"

echo "  Creating relic entity..."
RELIC=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Relic: $RELIC"


$STIGCTL component create "$HUNTER" HunterProfile '{
  "name": "Richter Belmont",
  "order": "Belmont Clan",
  "title": "Vampire Hunter",
  "renown": 87,
  "vows": [
    "Guard the lineage of the night",
    "Shield the innocent",
    "Banish the dark"
  ]
}' > /dev/null

$STIGCTL component create "$HUNTER" HunterStats '{
  "level": 12,
  "current_hp": 76,
  "max_hp": 92,
  "vitality": 22,
  "strength": 24,
  "focus": 18,
  "resilience": 20,
  "speed": 19,
  "resolve": 30,
  "status": "ready",
  "experience": 3400,
  "experience_to_next_rank": 1200
}' > /dev/null

$STIGCTL component create "$HUNTER" Arsenal '{
  "primary_weapon": "Vampire Killer Whip",
  "backup_weapons": ["Throwing Dagger", "Consecrated Axe"],
  "active_relics": ["Morning Star", "Crimson Vial"]
}' > /dev/null

$STIGCTL component create "$HUNTER" Inventory '{
  "items": {
    "healing_vial": 4,
    "holy_water": 3,
    "throwing_dagger": 12,
    "enchanted_cross": 1
  }
}' > /dev/null

$STIGCTL component create "$HUNTER" Location '{
  "region": "Transylvania",
  "area": "Outer Keep",
  "x": 12.5,
  "y": -3.0,
  "altitude": 480.0
}' > /dev/null

$STIGCTL component create "$HUNTER" QuestLog '{
  "active": [
    "Investigate the crimson moon",
    "Recover the shattered rosary"
  ],
  "completed": [
    "Clear the village catacombs"
  ]
}' > /dev/null

$STIGCTL component create "$MENTOR" Mentor '{
  "name": "Sister Maria",
  "role": "Archivist of the Holy Order",
  "dialogue": [
    "The castle shifts with every eclipse.",
    "Listen to the bells; they warn of the lord'"'"'s awakening.",
    "A relic sleeps in the librar'"'"'s forbidden wing."
  ],
  "offers_training": true,
  "location_hint": "Sanctuary library"
}' > /dev/null

$STIGCTL component create "$MENTOR" Location '{
  "region": "Transylvania",
  "area": "Sanctuary Library",
  "x": -2.0,
  "y": 14.0,
  "altitude": 220.0
}' > /dev/null

$STIGCTL component create "$MONSTER" MonsterProfile '{
  "name": "Nightshade Revenant",
  "species": "Spectral Warlord",
  "threat_level": "greater",
  "origin": "Rotting battlefield mists",
  "weaknesses": ["Consecrated fire", "Sun sigils"],
  "lair": "Ruined battlements",
  "description": "A commander slain centuries ago, now bound to dusk." 
}' > /dev/null

$STIGCTL component create "$MONSTER" MonsterState '{
  "current_hp": 210,
  "max_hp": 210,
  "status": "lurking",
  "aggression": 65,
  "enrage": 25
}' > /dev/null

$STIGCTL component create "$MONSTER" Location '{
  "region": "Transylvania",
  "area": "Ruined Battlements",
  "x": 48.0,
  "y": 7.5,
  "altitude": 960.0
}' > /dev/null

$STIGCTL component create "$RELIC" Relic '{
  "name": "Solar Sigil",
  "category": "blessing",
  "description": "A radiant emblem that weakens night-bound foes.",
  "power": "Imbues strikes with searing light"
}' > /dev/null

$STIGCTL component create "$RELIC" Location '{
  "region": "Transylvania",
  "area": "Clocktower Apex",
  "x": 3.0,
  "y": 88.0,
  "altitude": 1220.0
}' > /dev/null

echo ""
echo "================================================"
echo "Castlevania World Setup Complete!"
echo "================================================"
echo ""
echo "Entity IDs:"
echo "  Hunter:          $HUNTER"
echo "  Mentor:          $MENTOR"
echo "  Monster:         $MONSTER"
echo "  Relic Location:  $RELIC"
echo ""
echo "Try listing entities with:"
echo "  $STIGCTL entity list"
echo ""
echo "Inspect the hunter with:"
echo "  $STIGCTL component list $HUNTER"
echo ""
echo "Use castlevania-examples.sh to interact with the world."
echo ""
