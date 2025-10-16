#!/usr/bin/env bash
# A dramatic adventure inside the Castlevania-inspired world
# Run setup-castlevania-world.sh first

set -euo pipefail

# Build stigctl once at the start to avoid cargo noise later
echo "Building stigctl..."
cargo build --bin stigctl 2>&1 | grep -v "Compiling\|Finished" || true
echo ""

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

pause() {
    sleep "${1:-2}"
}

divider() {
    echo ""
    echo "================================"
    echo ""
}

clear

cat <<'EOF'
╔══════════════════════════════════════════════════════╗
║                                                      ║
║          BALLAD OF THE VEIL-BREAKER                  ║
║                                                      ║
║                A Stigmergy Chronicle                 ║
║                                                      ║
╚══════════════════════════════════════════════════════╝

EOF

pause 2

echo "Moonlight spills through shattered stained glass..."
pause 2
echo "Tonight, a lone hunter crosses the castle threshold."
pause 2
divider

echo ">> Summoning the Veil-Breaker..."
HERO=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
pause 1

$STIGCTL component create "$HERO" HunterProfile '{
  "name": "Elena Belnades",
  "order": "Order of the Veil",
  "title": "Veil-Breaker",
  "renown": 72,
  "vows": [
    "Carry dawn into every corridor",
    "Guard the last sanctuaries",
    "Silence the songs of night"
  ]
}' > /dev/null

$STIGCTL component create "$HERO" HunterStats '{
  "level": 11,
  "current_hp": 68,
  "max_hp": 88,
  "vitality": 21,
  "strength": 19,
  "focus": 24,
  "resilience": 18,
  "speed": 20,
  "resolve": 28,
  "status": "ready",
  "experience": 2950,
  "experience_to_next_rank": 950
}' > /dev/null

$STIGCTL component create "$HERO" Arsenal '{
  "primary_weapon": "Auric Chainwhip",
  "backup_weapons": ["Crystal Chakram", "Sanctified Pike"],
  "active_relics": ["Lunar Prayer"]
}' > /dev/null

$STIGCTL component create "$HERO" Inventory '{
  "items": {
    "healing_vial": 3,
    "sun_scroll": 1,
    "etheric_coil": 2
  }
}' > /dev/null

$STIGCTL component create "$HERO" Location '{
  "region": "Transylvania",
  "area": "Hall of Echoes",
  "x": -6.0,
  "y": 18.5,
  "altitude": 360.0
}' > /dev/null

$STIGCTL component create "$HERO" QuestLog '{
  "active": [],
  "completed": ["Secure the mirror shrine"]
}' > /dev/null

echo "Elena Belnades steps onto the marble floor."
pause 2
divider

echo ">> A cloaked archivist waits beside a ruined lectern..."
MENTOR=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
pause 1

$STIGCTL component create "$MENTOR" Mentor '{
  "name": "Archivist Lucian",
  "role": "Chronicler of eclipses",
  "dialogue": [
    "The castle sings tonight; listen for dissonance.",
    "A relic of dawn resonates beyond the choir loft.",
    "Seal the velvet nocturne before it devours the choir."
  ],
  "offers_training": false,
  "location_hint": "Choir loft"
}' > /dev/null

$STIGCTL component create "$MENTOR" Location '{
  "region": "Transylvania",
  "area": "Hall of Echoes",
  "x": -4.5,
  "y": 17.0,
  "altitude": 360.0
}' > /dev/null

echo "Archvist Lucian: 'Hunter, the choir bleeds shadow. You must seal it.'"
pause 3

QUEST_LOG=$($STIGCTL component get "$HERO" QuestLog 2>/dev/null)
UPDATED_QUEST=$(echo "$QUEST_LOG" | jq '.active = ((.active // []) + ["Seal the velvet nocturne"] | unique)')
$STIGCTL component update "$HERO" QuestLog "$UPDATED_QUEST" > /dev/null

echo "Quest Added: Seal the velvet nocturne"
pause 2
divider

echo ">> A relic hums beyond shattered pews..."
RELIC=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
pause 1

$STIGCTL component create "$RELIC" Relic '{
  "name": "Prismatic Canticle",
  "category": "artifact",
  "description": "A lattice of glass that refracts hymns into searing light.",
  "power": "Amplifies relic harmonies"
}' > /dev/null

$STIGCTL component create "$RELIC" Location '{
  "region": "Transylvania",
  "area": "Choir Loft",
  "x": 9.0,
  "y": 27.0,
  "altitude": 420.0
}' > /dev/null

echo "Elena approaches the Prismatic Canticle."
pause 2

echo "She raises the relic, feeling warmth surge through runes."
pause 2

ARSENAL=$($STIGCTL component get "$HERO" Arsenal 2>/dev/null)
UPDATED_ARSENAL=$(echo "$ARSENAL" | jq '.active_relics += ["Prismatic Canticle"] | .active_relics |= unique')
$STIGCTL component update "$HERO" Arsenal "$UPDATED_ARSENAL" > /dev/null

echo "Relic Equipped: Prismatic Canticle"
pause 2
divider

echo ">> The velvet nocturne unfurls from the ruined organ pipes..."
MONSTER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
pause 1

$STIGCTL component create "$MONSTER" MonsterProfile '{
  "name": "Velvet Nocturne",
  "species": "Choir Wraith",
  "threat_level": "nightmare",
  "origin": "Echoes of forsaken hymns",
  "weaknesses": ["Prismatic light", "Consecrated resonance"],
  "lair": "Choir Loft",
  "description": "A symphony of lost voices bound into predatory dusk."
}' > /dev/null

$STIGCTL component create "$MONSTER" MonsterState '{
  "current_hp": 360,
  "max_hp": 360,
  "status": "rampaging",
  "aggression": 80,
  "enrage": 55
}' > /dev/null

$STIGCTL component create "$MONSTER" Location '{
  "region": "Transylvania",
  "area": "Choir Loft",
  "x": 10.0,
  "y": 28.5,
  "altitude": 420.0
}' > /dev/null

echo "The wraith screams, shattering lingering light."
pause 2

echo "Battle begins!"
pause 2

divider

echo "Round 1: Elena channels the Canticle."
pause 1

MONSTER_STATE=$($STIGCTL component get "$MONSTER" MonsterState 2>/dev/null)
MONSTER_STATE=$(echo "$MONSTER_STATE" | jq '.current_hp -= 90 | .aggression += 5')
$STIGCTL component update "$MONSTER" MonsterState "$MONSTER_STATE" > /dev/null

echo "The wraith recoils as 90 harm sears its essence."
pause 2

HERO_STATS=$($STIGCTL component get "$HERO" HunterStats 2>/dev/null)
HERO_STATS=$(echo "$HERO_STATS" | jq '.current_hp -= 26 | .status = "wounded"')
$STIGCTL component update "$HERO" HunterStats "$HERO_STATS" > /dev/null

echo "Elena staggers, losing 26 vitality."
pause 2

divider

echo "Round 2: Elena drinks a healing vial."
pause 1

INVENTORY=$($STIGCTL component get "$HERO" Inventory 2>/dev/null)
UPDATED_INVENTORY=$(echo "$INVENTORY" | jq '.items.healing_vial = (((.items.healing_vial // 0) - 1) | if . < 0 then 0 else . end)')
$STIGCTL component update "$HERO" Inventory "$UPDATED_INVENTORY" > /dev/null

HERO_STATS=$($STIGCTL component get "$HERO" HunterStats 2>/dev/null)
HERO_STATS=$(echo "$HERO_STATS" | jq '.current_hp = ([.current_hp + 32, .max_hp] | min) | .status = (if .current_hp == .max_hp then "ready" else "wounded" end)')
$STIGCTL component update "$HERO" HunterStats "$HERO_STATS" > /dev/null

echo "Healing Vial consumed. Elena's wounds knit with radiant threads."
pause 2

divider

echo "Round 3: Finale. Elena weaves the Canticle's chord."
pause 1

MONSTER_STATE=$($STIGCTL component get "$MONSTER" MonsterState 2>/dev/null)
MONSTER_STATE=$(echo "$MONSTER_STATE" | jq '.current_hp = 0 | .status = "banished" | .aggression = 0 | .enrage = 0')
$STIGCTL component update "$MONSTER" MonsterState "$MONSTER_STATE" > /dev/null

echo "The Velvet Nocturne dissolves into resonant motes."
pause 2

divider

echo "Quest Complete."
QUEST_LOG=$($STIGCTL component get "$HERO" QuestLog 2>/dev/null)
QUEST_LOG=$(echo "$QUEST_LOG" | jq '
    .active = ((.active // []) | [ .[] | select(. != "Seal the velvet nocturne") ]) |
    .completed = ((.completed // []) + ["Seal the velvet nocturne"])
')
$STIGCTL component update "$HERO" QuestLog "$QUEST_LOG" > /dev/null

echo "Archivist Lucian bows, relief softening his eyes."
pause 2

divider

cat <<'EOF'
╔══════════════════════════════════════════════════════╗
║                  ENCOUNTER SUMMARY                    ║
╚══════════════════════════════════════════════════════╝
EOF

FINAL_STATS=$($STIGCTL component get "$HERO" HunterStats 2>/dev/null)
FINAL_INV=$($STIGCTL component get "$HERO" Inventory 2>/dev/null)
FINAL_QUESTS=$($STIGCTL component get "$HERO" QuestLog 2>/dev/null)

printf "%-18s %s\n" "Hunter:" "$HERO"
printf "%-18s %s/%s\n" "Current HP:" "$(echo "$FINAL_STATS" | jq -r '.current_hp')" "$(echo "$FINAL_STATS" | jq -r '.max_hp')"
printf "%-18s %s\n" "Status:" "$(echo "$FINAL_STATS" | jq -r '.status')"
printf "%-18s %s\n" "Healing Vials:" "$(echo "$FINAL_INV" | jq -r '.items.healing_vial // 0')"
printf "%-18s %s\n" "Completed Quests:" "$(echo "$FINAL_QUESTS" | jq -r '.completed | length')"

cat <<'EOF'
╔══════════════════════════════════════════════════════╗
║          The choir hums with dawn once more.          ║
║           New echoes await beyond the nave.           ║
╚══════════════════════════════════════════════════════╝
EOF

echo ""
echo "Entities remain in the database for further adventures."
echo "Use castlevania-examples.sh to continue Elena's campaign."
echo ""
