#!/usr/bin/env bash
set -euo pipefail

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

show_usage() {
    cat <<EOF
Pokevania World Examples
========================

Usage: $0 <command> [args...]

Commands:
  list-entities                                List all entities in the world
  show-hybrid-trainer <entity-id>              Show trainer with both Hunter and Trainer components
  show-gothic-pokemon <entity-id>              Show Pokemon with both Pokemon and Monster components
  show-hybrid-npc <entity-id>                  Show NPC with Mentor/Trainer/Hunter components
  show-hybrid-item <entity-id>                 Show Item with both Relic and Item components
  catch-gothic-pokemon <trainer-id> <pokemon-id>  Add a Gothic Pokemon to party
  equip-relic-to-pokemon <pokemon-id> <relic-name>  Enhance Pokemon with relic power
  gothic-battle <trainer-id> <pokemon-id>      Simulate battle (damages both sides)
  enrage-monster-pokemon <pokemon-id>          Increase aggression (makes catch harder)
  use-moon-potion <trainer-id> <pokemon-id>    Heal using hunter supplies on Pokemon
  train-with-mentor <trainer-id> <mentor-id>   Level up through mentor training
  challenge-gym <trainer-id> <leader-id>       Battle a hybrid gym leader
  create-gothic-pokemon <species> <threat>     Create new Gothic Pokemon (species: gengar|golbat|haunter|misdreavus|duskull, threat: lesser|greater|nightmare)

Examples:
  $0 list-entities
  $0 show-hybrid-trainer entity:AAAA...
  $0 catch-gothic-pokemon entity:TRAINER... entity:POKEMON...
  $0 create-gothic-pokemon misdreavus greater

EOF
}

list_entities() {
    echo "Listing all entities (Pokemon, Castlevania, and Pokevania)..."
    $STIGCTL entity list
}

show_hybrid_trainer() {
    local entity_id="$1"
    echo "=== HYBRID TRAINER (Hunter + Pokemon Trainer) ==="
    echo ""
    echo "--- Hunter Aspect ---"
    $STIGCTL component get "$entity_id" HunterProfile 2>/dev/null || echo "No HunterProfile"
    echo ""
    $STIGCTL component get "$entity_id" HunterStats 2>/dev/null || echo "No HunterStats"
    echo ""
    $STIGCTL component get "$entity_id" Arsenal 2>/dev/null || echo "No Arsenal"
    echo ""
    echo "--- Trainer Aspect ---"
    $STIGCTL component get "$entity_id" Trainer 2>/dev/null || echo "No Trainer"
    echo ""
    $STIGCTL component get "$entity_id" Party 2>/dev/null || echo "No Party"
    echo ""
    echo "--- Shared Components ---"
    $STIGCTL component get "$entity_id" Inventory 2>/dev/null || echo "No Inventory"
    echo ""
    $STIGCTL component get "$entity_id" Location 2>/dev/null || echo "No Location"
    echo ""
    $STIGCTL component get "$entity_id" QuestLog 2>/dev/null || echo "No QuestLog"
}

show_gothic_pokemon() {
    local entity_id="$1"
    echo "=== GOTHIC POKEMON (Pokemon + Monster) ==="
    echo ""
    echo "--- Pokemon Aspect ---"
    $STIGCTL component get "$entity_id" PokemonSpecies 2>/dev/null || echo "No PokemonSpecies"
    echo ""
    $STIGCTL component get "$entity_id" PokemonInstance 2>/dev/null || echo "No PokemonInstance"
    echo ""
    $STIGCTL component get "$entity_id" MoveSet 2>/dev/null || echo "No MoveSet"
    echo ""
    echo "--- Monster Aspect ---"
    $STIGCTL component get "$entity_id" MonsterProfile 2>/dev/null || echo "No MonsterProfile"
    echo ""
    $STIGCTL component get "$entity_id" MonsterState 2>/dev/null || echo "No MonsterState"
    echo ""
    echo "--- Shared Components ---"
    $STIGCTL component get "$entity_id" Location 2>/dev/null || echo "No Location"
}

show_hybrid_npc() {
    local entity_id="$1"
    echo "=== HYBRID NPC ==="
    echo ""
    $STIGCTL component get "$entity_id" NPC 2>/dev/null || echo "No NPC"
    echo ""
    $STIGCTL component get "$entity_id" Mentor 2>/dev/null || echo "No Mentor"
    echo ""
    $STIGCTL component get "$entity_id" Trainer 2>/dev/null || echo "No Trainer"
    echo ""
    $STIGCTL component get "$entity_id" HunterProfile 2>/dev/null || echo "No HunterProfile"
    echo ""
    $STIGCTL component get "$entity_id" Party 2>/dev/null || echo "No Party"
    echo ""
    $STIGCTL component get "$entity_id" Location 2>/dev/null || echo "No Location"
}

show_hybrid_item() {
    local entity_id="$1"
    echo "=== HYBRID ITEM (Relic + Item) ==="
    echo ""
    $STIGCTL component get "$entity_id" Relic 2>/dev/null || echo "No Relic"
    echo ""
    $STIGCTL component get "$entity_id" Item 2>/dev/null || echo "No Item"
    echo ""
    $STIGCTL component get "$entity_id" Location 2>/dev/null || echo "No Location"
}

catch_gothic_pokemon() {
    local trainer_id="$1"
    local pokemon_id="$2"

    echo "Attempting to catch Gothic Pokemon..."
    
    local monster_state
    monster_state=$($STIGCTL component get "$pokemon_id" MonsterState 2>/dev/null)
    local aggression
    aggression=$(echo "$monster_state" | jq -r '.aggression')
    
    echo "Monster aggression: $aggression"
    
    if [ "$aggression" -gt 50 ]; then
        echo "WARNING: High aggression! Using Dusk Ball for better catch rate..."
    fi
    
    echo "Weakening with Vampire Killer..."
    local updated_state
    updated_state=$(echo "$monster_state" | jq '.current_hp = (.current_hp * 0.6 | floor) | .aggression = (.aggression - 20)')
    $STIGCTL component update "$pokemon_id" MonsterState "$updated_state" 2>/dev/null
    
    echo "Throwing Dusk Ball..."
    sleep 1
    
    echo "Caught! Adding to party..."
    local party
    party=$($STIGCTL component get "$trainer_id" Party 2>/dev/null)
    local updated_party
    updated_party=$(echo "$party" | jq --arg pid "$pokemon_id" '.pokemon += [$pid]')
    $STIGCTL component update "$trainer_id" Party "$updated_party" 2>/dev/null
    
    local new_state
    new_state=$(echo "$updated_state" | jq '.status = "dormant" | .aggression = 10')
    $STIGCTL component update "$pokemon_id" MonsterState "$new_state" 2>/dev/null
    
    echo "Success! Gothic Pokemon added to party and calmed."
}

equip_relic_to_pokemon() {
    local pokemon_id="$1"
    local relic_name="$2"
    
    echo "Imbuing $pokemon_id with relic: $relic_name"
    
    local pokemon_inst
    pokemon_inst=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)
    
    case "$relic_name" in
        "Crimson Moon Fragment")
            echo "Boosting Ghost/Dark moves by 30%..."
            local updated_inst
            updated_inst=$(echo "$pokemon_inst" | jq '.sp_attack = (.sp_attack * 1.3 | floor)')
            $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_inst" 2>/dev/null
            echo "Special Attack boosted!"
            ;;
        "Solar Sigil")
            echo "Imbuing with holy light (weakens undead)..."
            local updated_inst
            updated_inst=$(echo "$pokemon_inst" | jq '.attack = (.attack * 1.2 | floor)')
            $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_inst" 2>/dev/null
            echo "Attack boosted against undead!"
            ;;
        *)
            echo "Unknown relic. No effect applied."
            ;;
    esac
}

gothic_battle() {
    local trainer_id="$1"
    local wild_pokemon_id="$2"
    
    echo "=== GOTHIC BATTLE ==="
    echo ""
    echo "Engaging wild monster-pokemon..."
    
    local hunter_stats
    hunter_stats=$($STIGCTL component get "$trainer_id" HunterStats 2>/dev/null)
    local trainer_hp
    trainer_hp=$(echo "$hunter_stats" | jq -r '.current_hp')
    local trainer_strength
    trainer_strength=$(echo "$hunter_stats" | jq -r '.strength')
    
    local monster_state
    monster_state=$($STIGCTL component get "$wild_pokemon_id" MonsterState 2>/dev/null)
    local monster_hp
    monster_hp=$(echo "$monster_state" | jq -r '.current_hp')
    local monster_aggression
    monster_aggression=$(echo "$monster_state" | jq -r '.aggression')
    
    echo "Trainer HP: $trainer_hp | Monster HP: $monster_hp | Aggression: $monster_aggression"
    echo ""
    
    echo "Round 1: Hunter strikes with Vampire Killer!"
    local damage=$((trainer_strength + 15))
    monster_hp=$((monster_hp - damage))
    echo "  Dealt $damage damage! Monster HP: $monster_hp"
    
    echo "Round 2: Monster retaliates!"
    local counter_damage=$((monster_aggression / 5 + 10))
    trainer_hp=$((trainer_hp - counter_damage))
    echo "  Took $counter_damage damage! Trainer HP: $trainer_hp"
    
    echo "Round 3: Final strike!"
    damage=$((trainer_strength + 20))
    monster_hp=$((monster_hp - damage))
    echo "  Critical hit! Dealt $damage damage! Monster HP: $monster_hp"
    
    echo ""
    if [ "$monster_hp" -le 0 ]; then
        echo "Monster defeated! Ready to catch."
        monster_hp=1
        monster_aggression=15
    else
        echo "Monster weakened and ready for capture."
        monster_aggression=$((monster_aggression - 30))
        if [ "$monster_aggression" -lt 10 ]; then
            monster_aggression=10
        fi
    fi
    
    local updated_hunter
    updated_hunter=$(echo "$hunter_stats" | jq --arg hp "$trainer_hp" '.current_hp = ($hp | tonumber)')
    $STIGCTL component update "$trainer_id" HunterStats "$updated_hunter" 2>/dev/null
    
    local updated_monster
    updated_monster=$(echo "$monster_state" | jq --arg hp "$monster_hp" --arg agg "$monster_aggression" \
        '.current_hp = ($hp | tonumber) | .aggression = ($agg | tonumber) | .status = "lurking"')
    $STIGCTL component update "$wild_pokemon_id" MonsterState "$updated_monster" 2>/dev/null
    
    echo ""
    echo "Battle complete! Both entities updated."
}

enrage_monster_pokemon() {
    local pokemon_id="$1"
    
    echo "Provoking monster-pokemon..."
    
    local monster_state
    monster_state=$($STIGCTL component get "$pokemon_id" MonsterState 2>/dev/null)
    
    local updated_state
    updated_state=$(echo "$monster_state" | jq '.aggression = ((.aggression + 25) | if . > 100 then 100 else . end) | .enrage = ((.enrage + 40) | if . > 100 then 100 else . end) | .status = "rampaging"')
    
    $STIGCTL component update "$pokemon_id" MonsterState "$updated_state" 2>/dev/null
    
    echo "Monster enraged! Aggression and enrage levels increased."
    echo "Catching will be much harder now!"
}

use_moon_potion() {
    local trainer_id="$1"
    local pokemon_id="$2"
    
    echo "Using Moon Potion (hunter healing on Pokemon)..."
    
    local inventory
    inventory=$($STIGCTL component get "$trainer_id" Inventory 2>/dev/null)
    local moon_potion_count
    moon_potion_count=$(echo "$inventory" | jq -r '.items.moon_potion // 0')
    
    if [ "$moon_potion_count" -le 0 ]; then
        echo "ERROR: No Moon Potions left!"
        return 1
    fi
    
    local pokemon_inst
    pokemon_inst=$($STIGCTL component get "$pokemon_id" PokemonInstance 2>/dev/null)
    local max_hp
    max_hp=$(echo "$pokemon_inst" | jq -r '.max_hp')
    
    local updated_pokemon
    updated_pokemon=$(echo "$pokemon_inst" | jq --arg hp "$max_hp" '.current_hp = ($hp | tonumber)')
    $STIGCTL component update "$pokemon_id" PokemonInstance "$updated_pokemon" 2>/dev/null
    
    local monster_state
    monster_state=$($STIGCTL component get "$pokemon_id" MonsterState 2>/dev/null)
    local updated_monster
    updated_monster=$(echo "$monster_state" | jq --arg hp "$max_hp" '.current_hp = ($hp | tonumber)')
    $STIGCTL component update "$pokemon_id" MonsterState "$updated_monster" 2>/dev/null
    
    local updated_inventory
    updated_inventory=$(echo "$inventory" | jq '.items.moon_potion -= 1')
    $STIGCTL component update "$trainer_id" Inventory "$updated_inventory" 2>/dev/null
    
    echo "Pokemon fully healed! Moon Potions remaining: $((moon_potion_count - 1))"
}

train_with_mentor() {
    local trainer_id="$1"
    local mentor_id="$2"
    
    echo "Training with mentor..."
    
    local mentor
    mentor=$($STIGCTL component get "$mentor_id" Mentor 2>/dev/null)
    local mentor_name
    mentor_name=$(echo "$mentor" | jq -r '.name')
    
    echo "$mentor_name imparts ancient knowledge..."
    
    local hunter_stats
    hunter_stats=$($STIGCTL component get "$trainer_id" HunterStats 2>/dev/null)
    
    local updated_stats
    updated_stats=$(echo "$hunter_stats" | jq '
        .experience += 500 |
        .focus += 2 |
        .resolve += 3 |
        .max_hp += 5 |
        .current_hp = .max_hp
    ')
    
    $STIGCTL component update "$trainer_id" HunterStats "$updated_stats" 2>/dev/null
    
    echo "Training complete! Gained experience and permanent stat boosts."
}

challenge_gym() {
    local trainer_id="$1"
    local leader_id="$2"
    
    echo "=== GYM CHALLENGE ==="
    echo ""
    
    local npc
    npc=$($STIGCTL component get "$leader_id" NPC 2>/dev/null)
    local leader_name
    leader_name=$(echo "$npc" | jq -r '.name')
    local dialogue
    dialogue=$(echo "$npc" | jq -r '.dialogue[0]')
    
    echo "$leader_name: \"$dialogue\""
    echo ""
    sleep 1
    
    echo "Battle begins!"
    echo "..."
    sleep 1
    echo "Your Gothic Pokemon use a combination of type advantage and relic powers!"
    echo "..."
    sleep 1
    echo "Victory!"
    echo ""
    
    local updated_npc
    updated_npc=$(echo "$npc" | jq '.defeated = true')
    $STIGCTL component update "$leader_id" NPC "$updated_npc" 2>/dev/null
    
    local trainer
    trainer=$($STIGCTL component get "$trainer_id" Trainer 2>/dev/null)
    local badge_name
    badge_name="Shadow Badge"
    
    local updated_trainer
    updated_trainer=$(echo "$trainer" | jq --arg badge "$badge_name" '.badges += [$badge] | .money += 5000')
    $STIGCTL component update "$trainer_id" Trainer "$updated_trainer" 2>/dev/null
    
    echo "$leader_name: \"Impressive... Take this Shadow Badge. You have earned it.\""
    echo ""
    echo "Received Shadow Badge and 5000 money!"
}

create_gothic_pokemon() {
    local species="$1"
    local threat="${2:-greater}"
    
    echo "Creating Gothic $species (threat: $threat)..."
    
    local entity
    entity=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
    
    case "$species" in
        "gengar")
            $STIGCTL component create "$entity" PokemonSpecies '{
              "name": "Gengar",
              "pokedex_number": 94,
              "primary_type": "Ghost",
              "secondary_type": "Poison",
              "base_hp": 60, "base_attack": 65, "base_defense": 60,
              "base_sp_attack": 130, "base_sp_defense": 75, "base_speed": 110,
              "evolution_level": null, "evolves_into": null
            }' > /dev/null
            
            $STIGCTL component create "$entity" PokemonInstance '{
              "nickname": null, "level": 30, "current_hp": 120, "max_hp": 120,
              "attack": 85, "defense": 78, "sp_attack": 160, "sp_defense": 95, "speed": 140,
              "experience": 27000, "experience_to_next_level": 3000,
              "status": null, "friendship": 50
            }' > /dev/null
            
            $STIGCTL component create "$entity" MoveSet '{
              "moves": ["Shadow Ball", "Sludge Bomb", "Dark Pulse", "Hypnosis"]
            }' > /dev/null
            
            $STIGCTL component create "$entity" MonsterProfile "{
              \"name\": \"Spectral Gengar\",
              \"species\": \"Shadow Phantom\",
              \"threat_level\": \"$threat\",
              \"origin\": \"Summoned from nightmare realm\",
              \"weaknesses\": [\"Psychic\", \"Ground\", \"Light magic\"],
              \"lair\": \"Shadow dimension\",
              \"description\": \"A Pokemon corrupted by dark castle energies.\"
            }" > /dev/null
            ;;
        "golbat")
            $STIGCTL component create "$entity" PokemonSpecies '{
              "name": "Golbat",
              "pokedex_number": 42,
              "primary_type": "Poison",
              "secondary_type": "Flying",
              "base_hp": 75, "base_attack": 80, "base_defense": 70,
              "base_sp_attack": 65, "base_sp_defense": 75, "base_speed": 90,
              "evolution_level": null, "evolves_into": "Crobat"
            }' > /dev/null
            
            $STIGCTL component create "$entity" PokemonInstance '{
              "nickname": null, "level": 25, "current_hp": 88, "max_hp": 88,
              "attack": 82, "defense": 72, "sp_attack": 68, "sp_defense": 78, "speed": 95,
              "experience": 15625, "experience_to_next_level": 2375,
              "status": null, "friendship": 50
            }' > /dev/null
            
            $STIGCTL component create "$entity" MoveSet '{
              "moves": ["Air Slash", "Poison Fang", "Bite", "Wing Attack"]
            }' > /dev/null
            
            $STIGCTL component create "$entity" MonsterProfile "{
              \"name\": \"Vampire Bat\",
              \"species\": \"Blood Drinker\",
              \"threat_level\": \"$threat\",
              \"origin\": \"Castle belfry\",
              \"weaknesses\": [\"Electric\", \"Ice\", \"Sunlight\"],
              \"lair\": \"Tower rafters\",
              \"description\": \"Feeds on the blood of the living.\"
            }" > /dev/null
            ;;
        "haunter")
            $STIGCTL component create "$entity" PokemonSpecies '{
              "name": "Haunter",
              "pokedex_number": 93,
              "primary_type": "Ghost",
              "secondary_type": "Poison",
              "base_hp": 45, "base_attack": 50, "base_defense": 45,
              "base_sp_attack": 115, "base_sp_defense": 55, "base_speed": 95,
              "evolution_level": null, "evolves_into": "Gengar"
            }' > /dev/null
            
            $STIGCTL component create "$entity" PokemonInstance '{
              "nickname": null, "level": 20, "current_hp": 62, "max_hp": 62,
              "attack": 42, "defense": 38, "sp_attack": 88, "sp_defense": 48, "speed": 72,
              "experience": 8000, "experience_to_next_level": 2000,
              "status": null, "friendship": 50
            }' > /dev/null
            
            $STIGCTL component create "$entity" MoveSet '{
              "moves": ["Lick", "Curse", "Night Shade", "Confuse Ray"]
            }' > /dev/null
            
            $STIGCTL component create "$entity" MonsterProfile "{
              \"name\": \"Wandering Spirit\",
              \"species\": \"Lesser Ghost\",
              \"threat_level\": \"$threat\",
              \"origin\": \"Graveyard mists\",
              \"weaknesses\": [\"Psychic\", \"Holy water\"],
              \"lair\": \"Tombstones\",
              \"description\": \"A restless spirit bound to the castle grounds.\"
            }" > /dev/null
            ;;
        "misdreavus")
            $STIGCTL component create "$entity" PokemonSpecies '{
              "name": "Misdreavus",
              "pokedex_number": 200,
              "primary_type": "Ghost",
              "secondary_type": null,
              "base_hp": 60, "base_attack": 60, "base_defense": 60,
              "base_sp_attack": 85, "base_sp_defense": 85, "base_speed": 85,
              "evolution_level": null, "evolves_into": "Mismagius"
            }' > /dev/null
            
            $STIGCTL component create "$entity" PokemonInstance '{
              "nickname": null, "level": 28, "current_hp": 82, "max_hp": 82,
              "attack": 62, "defense": 65, "sp_attack": 98, "sp_defense": 102, "speed": 95,
              "experience": 21952, "experience_to_next_level": 2048,
              "status": null, "friendship": 50
            }' > /dev/null
            
            $STIGCTL component create "$entity" MoveSet '{
              "moves": ["Psybeam", "Shadow Ball", "Pain Split", "Hex"]
            }' > /dev/null
            
            $STIGCTL component create "$entity" MonsterProfile "{
              \"name\": \"Wailing Banshee\",
              \"species\": \"Screaming Phantom\",
              \"threat_level\": \"$threat\",
              \"origin\": \"Echoes of the damned\",
              \"weaknesses\": [\"Dark\", \"Ghost\", \"Silence spells\"],
              \"lair\": \"Chapel ruins\",
              \"description\": \"Its screams drive men mad with terror.\"
            }" > /dev/null
            ;;
        "duskull")
            $STIGCTL component create "$entity" PokemonSpecies '{
              "name": "Duskull",
              "pokedex_number": 355,
              "primary_type": "Ghost",
              "secondary_type": null,
              "base_hp": 20, "base_attack": 40, "base_defense": 90,
              "base_sp_attack": 30, "base_sp_defense": 90, "base_speed": 25,
              "evolution_level": 37, "evolves_into": "Dusclops"
            }' > /dev/null
            
            $STIGCTL component create "$entity" PokemonInstance '{
              "nickname": null, "level": 15, "current_hp": 45, "max_hp": 45,
              "attack": 28, "defense": 52, "sp_attack": 22, "sp_defense": 55, "speed": 20,
              "experience": 3375, "experience_to_next_level": 1125,
              "status": null, "friendship": 50
            }' > /dev/null
            
            $STIGCTL component create "$entity" MoveSet '{
              "moves": ["Astonish", "Disable", "Pursuit"]
            }' > /dev/null
            
            $STIGCTL component create "$entity" MonsterProfile "{
              \"name\": \"Grim Reaper Child\",
              \"species\": \"Death Herald\",
              \"threat_level\": \"$threat\",
              \"origin\": \"Born from cemetery soil\",
              \"weaknesses\": [\"Ghost\", \"Dark\", \"Life force\"],
              \"lair\": \"Crypts\",
              \"description\": \"Said to guide lost souls to the afterlife.\"
            }" > /dev/null
            ;;
        *)
            echo "Unknown species. Use: gengar, golbat, haunter, misdreavus, or duskull"
            $STIGCTL entity delete "$entity" 2>/dev/null
            return 1
            ;;
    esac
    
    local hp
    case "$threat" in
        "nightmare") hp=200 ;;
        "greater") hp=120 ;;
        "lesser") hp=60 ;;
        *) hp=100 ;;
    esac
    
    $STIGCTL component create "$entity" MonsterState "{
      \"current_hp\": $hp,
      \"max_hp\": $hp,
      \"status\": \"lurking\",
      \"aggression\": $((RANDOM % 40 + 40)),
      \"enrage\": $((RANDOM % 30 + 10))
    }" > /dev/null
    
    $STIGCTL component create "$entity" Location '{
      "region": "Transylvania",
      "area": "Dark Forest",
      "x": 0.0,
      "y": 0.0,
      "altitude": 500.0
    }' > /dev/null
    
    echo "Created Gothic $species!"
    echo "Pokemon ID: $entity"
}

if [ $# -eq 0 ]; then
    show_usage
    exit 0
fi

COMMAND="$1"
shift

case "$COMMAND" in
    list-entities)
        list_entities
        ;;
    show-hybrid-trainer)
        show_hybrid_trainer "$@"
        ;;
    show-gothic-pokemon)
        show_gothic_pokemon "$@"
        ;;
    show-hybrid-npc)
        show_hybrid_npc "$@"
        ;;
    show-hybrid-item)
        show_hybrid_item "$@"
        ;;
    catch-gothic-pokemon)
        catch_gothic_pokemon "$@"
        ;;
    equip-relic-to-pokemon)
        equip_relic_to_pokemon "$@"
        ;;
    gothic-battle)
        gothic_battle "$@"
        ;;
    enrage-monster-pokemon)
        enrage_monster_pokemon "$@"
        ;;
    use-moon-potion)
        use_moon_potion "$@"
        ;;
    train-with-mentor)
        train_with_mentor "$@"
        ;;
    challenge-gym)
        challenge_gym "$@"
        ;;
    create-gothic-pokemon)
        create_gothic_pokemon "$@"
        ;;
    *)
        echo "Unknown command: $COMMAND"
        echo ""
        show_usage
        exit 1
        ;;
esac
