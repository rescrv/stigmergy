#!/usr/bin/env bash
# Example commands for interacting with the Castlevania-inspired world
# Run setup-castlevania-world.sh first to create the initial state

set -euo pipefail

# Use pre-built binary to avoid cargo output
STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

show_usage() {
    cat <<EOF
Castlevania World Examples
==========================

Usage: $0 <command> [args...]

Commands:
  list-entities                          List all entities in the world
  list-components <entity-id>            List all components for an entity
  show-hunter <entity-id>                Show hunter profile, stats, arsenal, inventory, quests
  show-monster <entity-id>               Show monster profile, state, and location
  scout-location <entity-id>             Inspect an entity's location component
  equip-relic <hunter-id> <relic-name>   Add a relic to the hunter's active relics
  collect-relic <hunter-id> <relic-id>   Fetch relic data by entity and equip it
  use-vial <hunter-id>                   Consume a healing vial to restore vitality
  record-quest <hunter-id> <quest>       Append a quest to the hunter's active log
  complete-quest <hunter-id> <quest>     Move an active quest to the completed list
  banish-monster <monster-id>            Reduce monster HP to zero and mark it banished
  create-monster <name> [threat]         Create a themed monster entity (threat: lesser|greater|nightmare)

Examples:
  $0 list-entities
  $0 show-hunter entity:HUNTER...
  $0 equip-relic entity:HUNTER... "Solar Sigil"
  $0 collect-relic entity:HUNTER... entity:RELIC...
  $0 create-monster "Bone Golem" nightmare

EOF
}

list_entities() {
    echo "Listing all entities..."
    $STIGCTL entity list
}

list_components() {
    local entity_id="$1"
    echo "Listing components for $entity_id..."
    $STIGCTL component list "$entity_id"
}

show_hunter() {
    local hunter_id="$1"
    echo "=== Hunter Profile ==="
    $STIGCTL component get "$hunter_id" HunterProfile 2>/dev/null
    echo ""
    echo "=== Hunter Stats ==="
    $STIGCTL component get "$hunter_id" HunterStats 2>/dev/null
    echo ""
    echo "=== Arsenal ==="
    $STIGCTL component get "$hunter_id" Arsenal 2>/dev/null
    echo ""
    echo "=== Inventory ==="
    $STIGCTL component get "$hunter_id" Inventory 2>/dev/null
    echo ""
    echo "=== Quest Log ==="
    $STIGCTL component get "$hunter_id" QuestLog 2>/dev/null
}

show_monster() {
    local monster_id="$1"
    echo "=== Monster Profile ==="
    $STIGCTL component get "$monster_id" MonsterProfile 2>/dev/null
    echo ""
    echo "=== Monster State ==="
    $STIGCTL component get "$monster_id" MonsterState 2>/dev/null
    echo ""
    echo "=== Location ==="
    $STIGCTL component get "$monster_id" Location 2>/dev/null
}

scout_location() {
    local entity_id="$1"
    echo "=== Location ==="
    $STIGCTL component get "$entity_id" Location 2>/dev/null
}

equip_relic() {
    local hunter_id="$1"
    local relic_name="$2"

    echo "Equipping relic: $relic_name"
    local arsenal_json
    arsenal_json=$($STIGCTL component get "$hunter_id" Arsenal 2>/dev/null)

    if [ -z "$arsenal_json" ]; then
        echo "Error: Hunter has no Arsenal component!"
        exit 1
    fi

    local updated
    updated=$(echo "$arsenal_json" | jq --arg relic "$relic_name" '
        if (.active_relics | index($relic)) == null then
            .active_relics += [$relic]
        else
            .
        end
    ')

    $STIGCTL component update "$hunter_id" Arsenal "$updated" 2>/dev/null
    echo "Relic equipped!"
    echo "$updated" | jq .
}

collect_relic() {
    local hunter_id="$1"
    local relic_entity="$2"

    echo "Collecting relic from $relic_entity..."
    local relic_json
    relic_json=$($STIGCTL component get "$relic_entity" Relic 2>/dev/null)

    if [ -z "$relic_json" ]; then
        echo "Error: Relic entity $relic_entity does not contain a Relic component!"
        exit 1
    fi

    local relic_name
    relic_name=$(echo "$relic_json" | jq -r '.name')

    echo "Found relic: $relic_name"
    equip_relic "$hunter_id" "$relic_name"
}

use_vial() {
    local hunter_id="$1"
    echo "Using healing vial..."

    local inventory_json
    inventory_json=$($STIGCTL component get "$hunter_id" Inventory 2>/dev/null)

    if [ -z "$inventory_json" ]; then
        echo "Error: Hunter has no Inventory component!"
        exit 1
    fi

    local vial_count
    vial_count=$(echo "$inventory_json" | jq '.items.healing_vial // 0')

    if [ "$vial_count" -le 0 ]; then
        echo "Error: No healing vials available!"
        exit 1
    fi

    local stats_json
    stats_json=$($STIGCTL component get "$hunter_id" HunterStats 2>/dev/null)

    if [ -z "$stats_json" ]; then
        echo "Error: Hunter has no HunterStats component!"
        exit 1
    fi

    local healed_stats
    healed_stats=$(echo "$stats_json" | jq '
        .current_hp = ([.current_hp + 30, .max_hp] | min) |
        .status = (if .current_hp == .max_hp then "ready" else .status end)
    ')

    local updated_inventory
    updated_inventory=$(echo "$inventory_json" | jq '
        .items.healing_vial = (((.items.healing_vial // 0) - 1) | if . < 0 then 0 else . end)
    ')

    $STIGCTL component update "$hunter_id" HunterStats "$healed_stats" 2>/dev/null
    $STIGCTL component update "$hunter_id" Inventory "$updated_inventory" 2>/dev/null

    echo "Restoration complete."
    echo "Remaining vials: $(echo "$updated_inventory" | jq -r '.items.healing_vial // 0')"
    echo "Current HP: $(echo "$healed_stats" | jq -r '.current_hp')/$(echo "$healed_stats" | jq -r '.max_hp')"
}

record_quest() {
    local hunter_id="$1"
    shift
    local quest="$*"

    if [ -z "$quest" ]; then
        echo "Error: Quest name is required."
        exit 1
    fi

    echo "Recording quest: $quest"
    local quest_json
    quest_json=$($STIGCTL component get "$hunter_id" QuestLog 2>/dev/null)

    if [ -z "$quest_json" ]; then
        echo "Error: Hunter has no QuestLog component!"
        exit 1
    fi

    local updated
    updated=$(echo "$quest_json" | jq --arg quest "$quest" '
        .active = (.active // []) |
        if (.active | index($quest)) == null then
            .active += [$quest]
        else
            .
        end
    ')

    $STIGCTL component update "$hunter_id" QuestLog "$updated" 2>/dev/null
    echo "Quest added to active log."
}

complete_quest() {
    local hunter_id="$1"
    shift
    local quest="$*"

    if [ -z "$quest" ]; then
        echo "Error: Quest name is required."
        exit 1
    fi

    echo "Completing quest: $quest"
    local quest_json
    quest_json=$($STIGCTL component get "$hunter_id" QuestLog 2>/dev/null)

    if [ -z "$quest_json" ]; then
        echo "Error: Hunter has no QuestLog component!"
        exit 1
    fi

    local updated
    updated=$(echo "$quest_json" | jq --arg quest "$quest" '
        .active = ((.active // []) | [ .[] | select(. != $quest) ]) |
        .completed = ((.completed // []) | if (. | index($quest)) == null then . + [$quest] else . end)
    ')

    $STIGCTL component update "$hunter_id" QuestLog "$updated" 2>/dev/null
    echo "Quest archived."
    echo "$updated" | jq .
}

banish_monster() {
    local monster_id="$1"
    echo "Banishing monster $monster_id..."

    local state_json
    state_json=$($STIGCTL component get "$monster_id" MonsterState 2>/dev/null)

    if [ -z "$state_json" ]; then
        echo "Error: MonsterState component missing."
        exit 1
    fi

    local updated
    updated=$(echo "$state_json" | jq '
        .current_hp = 0 |
        .status = "banished" |
        .aggression = 0 |
        .enrage = 0
    ')

    $STIGCTL component update "$monster_id" MonsterState "$updated" 2>/dev/null
    echo "Monster banished."
    echo "$updated" | jq '{status, current_hp, aggression, enrage}'
}

create_monster() {
    local monster_name="$1"
    local threat="${2:-lesser}"

    case "$threat" in
        lesser|greater|nightmare)
            ;;
        *)
            echo "Error: threat must be lesser, greater, or nightmare."
            exit 1
            ;;
    esac

    echo "Creating $threat threat monster: $monster_name"
    local monster_id
    monster_id=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

    local species origin weaknesses lair description current_hp max_hp aggression enrage region area altitude

    case "$threat" in
        lesser)
            species="Animated Armor"
            origin="Abandoned barracks"
            weaknesses='["Lightning", "Blessed steel"]'
            lair="Castle antechamber"
            description="Metal plates bound by restless souls."
            max_hp=120
            current_hp=120
            aggression=35
            enrage=10
            region="Transylvania"
            area="Castle Pass"
            altitude=410
            ;;
        greater)
            species="Shadow Behemoth"
            origin="Rift beneath the chapel"
            weaknesses='["Sun sigils", "Sacred bells"]'
            lair="Midnight chapel"
            description="A hulking shadow that devours moonlight."
            max_hp=240
            current_hp=240
            aggression=70
            enrage=40
            region="Transylvania"
            area="Moonlit Nave"
            altitude=620
            ;;
        nightmare)
            species="Eclipse Abomination"
            origin="Tear between astral planes"
            weaknesses='["Radiant prayers", "Consecrated flames", "Ancient runes"]'
            lair="Eclipse sanctum"
            description="A horror born from simultaneous eclipses."
            max_hp=380
            current_hp=380
            aggression=85
            enrage=65
            region="Transylvania"
            area="Eclipse Sanctum"
            altitude=1080
            ;;
    esac

    local profile_json
    profile_json=$(cat <<EOF
{
  "name": "$monster_name",
  "species": "$species",
  "threat_level": "$threat",
  "origin": "$origin",
  "weaknesses": $weaknesses,
  "lair": "$lair",
  "description": "$description"
}
EOF
)

    local state_json
    state_json=$(cat <<EOF
{
  "current_hp": $current_hp,
  "max_hp": $max_hp,
  "status": "lurking",
  "aggression": $aggression,
  "enrage": $enrage
}
EOF
)

    local location_json
    location_json=$(cat <<EOF
{
  "region": "$region",
  "area": "$area",
  "x": 0.0,
  "y": 0.0,
  "altitude": $altitude
}
EOF
)

    $STIGCTL component create "$monster_id" MonsterProfile "$profile_json" 2>/dev/null
    $STIGCTL component create "$monster_id" MonsterState "$state_json" 2>/dev/null
    $STIGCTL component create "$monster_id" Location "$location_json" 2>/dev/null

    echo "Monster created: $monster_id"
    echo "Inspect with: $0 show-monster $monster_id"
}

if [ $# -eq 0 ]; then
    show_usage
    exit 1
fi

case "$1" in
    list-entities)
        list_entities
        ;;
    list-components)
        [ $# -lt 2 ] && { echo "Error: Missing entity-id"; show_usage; exit 1; }
        list_components "$2"
        ;;
    show-hunter)
        [ $# -lt 2 ] && { echo "Error: Missing hunter-id"; show_usage; exit 1; }
        show_hunter "$2"
        ;;
    show-monster)
        [ $# -lt 2 ] && { echo "Error: Missing monster-id"; show_usage; exit 1; }
        show_monster "$2"
        ;;
    scout-location)
        [ $# -lt 2 ] && { echo "Error: Missing entity-id"; show_usage; exit 1; }
        scout_location "$2"
        ;;
    equip-relic)
        [ $# -lt 3 ] && { echo "Error: Missing hunter-id or relic-name"; show_usage; exit 1; }
        equip_relic "$2" "$3"
        ;;
    collect-relic)
        [ $# -lt 3 ] && { echo "Error: Missing hunter-id or relic-id"; show_usage; exit 1; }
        collect_relic "$2" "$3"
        ;;
    use-vial)
        [ $# -lt 2 ] && { echo "Error: Missing hunter-id"; show_usage; exit 1; }
        use_vial "$2"
        ;;
    record-quest)
        [ $# -lt 3 ] && { echo "Error: Missing hunter-id or quest"; show_usage; exit 1; }
        record_quest "$2" "${@:3}"
        ;;
    complete-quest)
        [ $# -lt 3 ] && { echo "Error: Missing hunter-id or quest"; show_usage; exit 1; }
        complete_quest "$2" "${@:3}"
        ;;
    banish-monster)
        [ $# -lt 2 ] && { echo "Error: Missing monster-id"; show_usage; exit 1; }
        banish_monster "$2"
        ;;
    create-monster)
        [ $# -lt 2 ] && { echo "Error: Missing monster name"; show_usage; exit 1; }
        create_monster "$2" "${3:-lesser}"
        ;;
    *)
        echo "Unknown command: $1"
        show_usage
        exit 1
        ;;
esac
