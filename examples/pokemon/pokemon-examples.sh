#!/usr/bin/env bash
# Example commands for interacting with the Pokemon world
# Run setup-pokemon-world.sh first to create the initial game state

set -euo pipefail

# Use pre-built binary to avoid cargo output
STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

show_usage() {
    cat <<EOF
Pokemon World Examples
======================

Usage: $0 <command> [args...]

Commands:
  list-entities                    List all entities in the world
  list-components <entity-id>      List all components for an entity
  show-trainer <entity-id>         Show trainer details
  show-pokemon <entity-id>         Show pokemon details (species + instance + moves)
  add-to-party <trainer-id> <pokemon-id>   Add a pokemon to trainer's party
  level-up <pokemon-id>            Level up a pokemon (increases stats)
  heal-pokemon <pokemon-id>        Fully heal a pokemon
  use-potion <trainer-id> <pokemon-id>     Use a potion on a pokemon
  catch-pokemon <trainer-id>       Simulate catching a new pokemon (creates Rattata)
  battle-damage <pokemon-id> <damage>      Apply damage to a pokemon
  create-wild-pokemon <name>       Create a new wild pokemon entity

Examples:
  $0 list-entities
  $0 show-pokemon entity:AAAA...
  $0 add-to-party entity:PLAYER... entity:POKEMON...
  $0 level-up entity:POKEMON...

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

show_trainer() {
    local entity_id="$1"
    echo "=== Trainer Info ==="
    $STIGCTL component get "$entity_id" Trainer 2>/dev/null
    echo ""
    echo "=== Position ==="
    $STIGCTL component get "$entity_id" Position 2>/dev/null
    echo ""
    echo "=== Party ==="
    $STIGCTL component get "$entity_id" Party 2>/dev/null
    echo ""
    echo "=== Inventory ==="
    $STIGCTL component get "$entity_id" Inventory 2>/dev/null
}

show_pokemon() {
    local entity_id="$1"
    echo "=== Pokemon Species ==="
    $STIGCTL component get "$entity_id" PokemonSpecies 2>/dev/null
    echo ""
    echo "=== Pokemon Stats ==="
    $STIGCTL component get "$entity_id" PokemonInstance 2>/dev/null
    echo ""
    echo "=== Moveset ==="
    $STIGCTL component get "$entity_id" MoveSet 2>/dev/null
}

add_to_party() {
    local trainer_id="$1"
    local pokemon_id="$2"

    echo "Fetching current party..."
    local current_party=$($STIGCTL component get "$trainer_id" Party 2>/dev/null)

    echo "Adding $pokemon_id to party..."
    local updated_party=$(echo "$current_party" | jq --arg pid "$pokemon_id" '.pokemon += [$pid]')

    $STIGCTL component update "$trainer_id" Party "$updated_party" 2>/dev/null
    echo "Pokemon added to party!"
}

level_up() {
    local pokemon_id="$1"

    echo "Fetching current stats..."
    local current_stats=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)

    echo "Leveling up pokemon..."
    local updated_stats=$(echo "$current_stats" | jq '
        .level += 1 |
        .max_hp += 3 |
        .current_hp = .max_hp |
        .attack += 2 |
        .defense += 2 |
        .sp_attack += 2 |
        .sp_defense += 2 |
        .speed += 2 |
        .experience = 0 |
        .experience_to_next_level = (.level * 125)
    ')

    $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_stats" 2>/dev/null
    echo "Pokemon leveled up!"
    echo "$updated_stats" | jq .
}

heal_pokemon() {
    local pokemon_id="$1"

    echo "Healing pokemon..."
    local current_stats=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)
    local updated_stats=$(echo "$current_stats" | jq '.current_hp = .max_hp | .status = null')

    $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_stats" 2>/dev/null
    echo "Pokemon fully healed!"
}

use_potion() {
    local trainer_id="$1"
    local pokemon_id="$2"

    echo "Using potion..."

    local inventory=$($STIGCTL component get "$trainer_id" Inventory 2>/dev/null)
    local potion_count=$(echo "$inventory" | jq '.items.potion // 0')

    if [ "$potion_count" -le 0 ]; then
        echo "Error: No potions in inventory!"
        exit 1
    fi

    local pokemon_stats=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)
    local updated_stats=$(echo "$pokemon_stats" | jq '
        .current_hp = ([.current_hp + 20, .max_hp] | min)
    ')

    local updated_inventory=$(echo "$inventory" | jq '.items.potion -= 1')

    $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_stats" 2>/dev/null
    $STIGCTL component update "$trainer_id" Inventory "$updated_inventory" 2>/dev/null

    echo "Potion used! Pokemon healed by 20 HP."
    echo "Remaining potions: $((potion_count - 1))"
}

battle_damage() {
    local pokemon_id="$1"
    local damage="$2"

    echo "Applying $damage damage to pokemon..."
    local current_stats=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)
    local updated_stats=$(echo "$current_stats" | jq --arg dmg "$damage" '
        .current_hp = ((.current_hp - ($dmg | tonumber)) | if . < 0 then 0 else . end)
    ')

    $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_stats" 2>/dev/null
    echo "Damage applied!"
    echo "$updated_stats" | jq '{level, current_hp, max_hp, status}'
}

catch_pokemon() {
    local trainer_id="$1"

    echo "Creating wild Rattata..."
    local new_pokemon=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
    echo "New pokemon entity: $new_pokemon"

    $STIGCTL component create "$new_pokemon" PokemonSpecies '{
      "name": "Rattata",
      "pokedex_number": 19,
      "primary_type": "Normal",
      "secondary_type": null,
      "base_hp": 30,
      "base_attack": 56,
      "base_defense": 35,
      "base_sp_attack": 25,
      "base_sp_defense": 35,
      "base_speed": 72,
      "evolution_level": 20,
      "evolves_into": "Raticate"
    }'

    $STIGCTL component create "$new_pokemon" PokemonInstance '{
      "nickname": null,
      "level": 3,
      "current_hp": 14,
      "max_hp": 14,
      "attack": 9,
      "defense": 7,
      "sp_attack": 6,
      "sp_defense": 7,
      "speed": 12,
      "experience": 0,
      "experience_to_next_level": 75,
      "status": null,
      "friendship": 50
    }'

    $STIGCTL component create "$new_pokemon" MoveSet '{
      "moves": ["Tackle", "Tail Whip"]
    }'

    echo "Adding to party..."
    local current_party=$($STIGCTL component get "$trainer_id" Party 2>/dev/null)
    local updated_party=$(echo "$current_party" | jq --arg pid "$new_pokemon" '.pokemon += [$pid]')
    $STIGCTL component update "$trainer_id" Party "$updated_party" 2>/dev/null

    echo "Caught Rattata! Added to party."
    echo "Pokemon ID: $new_pokemon"
}

create_wild_pokemon() {
    local pokemon_name="$1"

    echo "Creating wild pokemon: $pokemon_name"
    local new_pokemon=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
    echo "New pokemon entity: $new_pokemon"

    case "$pokemon_name" in
        "pidgey"|"Pidgey")
            $STIGCTL component create "$new_pokemon" PokemonSpecies '{
              "name": "Pidgey",
              "pokedex_number": 16,
              "primary_type": "Normal",
              "secondary_type": "Flying",
              "base_hp": 40,
              "base_attack": 45,
              "base_defense": 40,
              "base_sp_attack": 35,
              "base_sp_defense": 35,
              "base_speed": 56,
              "evolution_level": 18,
              "evolves_into": "Pidgeotto"
            }'
            $STIGCTL component create "$new_pokemon" PokemonInstance '{
              "nickname": null,
              "level": 5,
              "current_hp": 19,
              "max_hp": 19,
              "attack": 10,
              "defense": 9,
              "sp_attack": 8,
              "sp_defense": 8,
              "speed": 12,
              "experience": 0,
              "experience_to_next_level": 125,
              "status": null,
              "friendship": 50
            }'
            $STIGCTL component create "$new_pokemon" MoveSet '{
              "moves": ["Tackle", "Sand Attack"]
            }'
            ;;
        "pikachu"|"Pikachu")
            $STIGCTL component create "$new_pokemon" PokemonSpecies '{
              "name": "Pikachu",
              "pokedex_number": 25,
              "primary_type": "Electric",
              "secondary_type": null,
              "base_hp": 35,
              "base_attack": 55,
              "base_defense": 40,
              "base_sp_attack": 50,
              "base_sp_defense": 50,
              "base_speed": 90,
              "evolution_level": null,
              "evolves_into": null
            }'
            $STIGCTL component create "$new_pokemon" PokemonInstance '{
              "nickname": null,
              "level": 5,
              "current_hp": 18,
              "max_hp": 18,
              "attack": 11,
              "defense": 9,
              "sp_attack": 11,
              "sp_defense": 11,
              "speed": 18,
              "experience": 0,
              "experience_to_next_level": 125,
              "status": null,
              "friendship": 50
            }'
            $STIGCTL component create "$new_pokemon" MoveSet '{
              "moves": ["Thunder Shock", "Growl", "Tail Whip"]
            }'
            ;;
        *)
            echo "Unknown pokemon: $pokemon_name"
            echo "Available: pidgey, pikachu"
            exit 1
            ;;
    esac

    echo "Created wild $pokemon_name!"
    echo "Pokemon ID: $new_pokemon"
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
    show-trainer)
        [ $# -lt 2 ] && { echo "Error: Missing entity-id"; show_usage; exit 1; }
        show_trainer "$2"
        ;;
    show-pokemon)
        [ $# -lt 2 ] && { echo "Error: Missing entity-id"; show_usage; exit 1; }
        show_pokemon "$2"
        ;;
    add-to-party)
        [ $# -lt 3 ] && { echo "Error: Missing trainer-id or pokemon-id"; show_usage; exit 1; }
        add_to_party "$2" "$3"
        ;;
    level-up)
        [ $# -lt 2 ] && { echo "Error: Missing pokemon-id"; show_usage; exit 1; }
        level_up "$2"
        ;;
    heal-pokemon)
        [ $# -lt 2 ] && { echo "Error: Missing pokemon-id"; show_usage; exit 1; }
        heal_pokemon "$2"
        ;;
    use-potion)
        [ $# -lt 3 ] && { echo "Error: Missing trainer-id or pokemon-id"; show_usage; exit 1; }
        use_potion "$2" "$3"
        ;;
    battle-damage)
        [ $# -lt 3 ] && { echo "Error: Missing pokemon-id or damage"; show_usage; exit 1; }
        battle_damage "$2" "$3"
        ;;
    catch-pokemon)
        [ $# -lt 2 ] && { echo "Error: Missing trainer-id"; show_usage; exit 1; }
        catch_pokemon "$2"
        ;;
    create-wild-pokemon)
        [ $# -lt 2 ] && { echo "Error: Missing pokemon name"; show_usage; exit 1; }
        create_wild_pokemon "$2"
        ;;
    *)
        echo "Unknown command: $1"
        show_usage
        exit 1
        ;;
esac
