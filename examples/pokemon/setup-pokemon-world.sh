#!/usr/bin/env bash
# Setup script for a Pokemon-like game world using Stigmergy
# This creates component definitions and initial game entities

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

echo "Creating component definitions..."

echo "  Creating PokemonSpecies component..."
$STIGCTL componentdefinition create PokemonSpecies '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "pokedex_number": {"type": "integer"},
    "primary_type": {"type": "string", "enum": ["Normal", "Fire", "Water", "Grass", "Electric", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon", "Dark", "Steel", "Fairy"]},
    "secondary_type": {"oneOf": [{"type": "string", "enum": ["Normal", "Fire", "Water", "Grass", "Electric", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon", "Dark", "Steel", "Fairy"]}, {"type": "null"}]},
    "base_hp": {"type": "integer"},
    "base_attack": {"type": "integer"},
    "base_defense": {"type": "integer"},
    "base_sp_attack": {"type": "integer"},
    "base_sp_defense": {"type": "integer"},
    "base_speed": {"type": "integer"},
    "evolution_level": {"oneOf": [{"type": "integer"}, {"type": "null"}]},
    "evolves_into": {"oneOf": [{"type": "string"}, {"type": "null"}]}
  },
  "required": ["name", "pokedex_number", "primary_type", "base_hp", "base_attack", "base_defense", "base_sp_attack", "base_sp_defense", "base_speed"]
}'

echo "  Creating PokemonInstance component..."
$STIGCTL componentdefinition create PokemonInstance '{
  "type": "object",
  "properties": {
    "nickname": {"oneOf": [{"type": "string"}, {"type": "null"}]},
    "level": {"type": "integer", "minimum": 1, "maximum": 100},
    "current_hp": {"type": "integer"},
    "max_hp": {"type": "integer"},
    "attack": {"type": "integer"},
    "defense": {"type": "integer"},
    "sp_attack": {"type": "integer"},
    "sp_defense": {"type": "integer"},
    "speed": {"type": "integer"},
    "experience": {"type": "integer"},
    "experience_to_next_level": {"type": "integer"},
    "status": {"oneOf": [{"type": "string", "enum": ["healthy", "poisoned", "burned", "paralyzed", "frozen", "asleep"]}, {"type": "null"}]},
    "friendship": {"type": "integer", "minimum": 0, "maximum": 255}
  },
  "required": ["level", "current_hp", "max_hp", "attack", "defense", "sp_attack", "sp_defense", "speed", "experience", "experience_to_next_level", "friendship"]
}'

echo "  Creating Move component..."
$STIGCTL componentdefinition create Move '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "move_type": {"type": "string", "enum": ["Normal", "Fire", "Water", "Grass", "Electric", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon", "Dark", "Steel", "Fairy"]},
    "category": {"type": "string", "enum": ["Physical", "Special", "Status"]},
    "power": {"oneOf": [{"type": "integer"}, {"type": "null"}]},
    "accuracy": {"oneOf": [{"type": "integer"}, {"type": "null"}]},
    "pp": {"type": "integer"},
    "max_pp": {"type": "integer"},
    "description": {"type": "string"}
  },
  "required": ["name", "move_type", "category", "pp", "max_pp", "description"]
}'

echo "  Creating MoveSet component..."
$STIGCTL componentdefinition create MoveSet '{
  "type": "object",
  "properties": {
    "moves": {
      "type": "array",
      "items": {"type": "string"},
      "maxItems": 4
    }
  },
  "required": ["moves"]
}'

echo "  Creating Position component..."
$STIGCTL componentdefinition create Position '{
  "type": "object",
  "properties": {
    "x": {"type": "number"},
    "y": {"type": "number"},
    "map": {"type": "string"},
    "facing": {"type": "string", "enum": ["north", "south", "east", "west"]}
  },
  "required": ["x", "y", "map", "facing"]
}'

echo "  Creating Trainer component..."
$STIGCTL componentdefinition create Trainer '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "money": {"type": "integer", "minimum": 0},
    "badges": {
      "type": "array",
      "items": {"type": "string"}
    },
    "trainer_id": {"type": "string"}
  },
  "required": ["name", "money", "badges", "trainer_id"]
}'

echo "  Creating Party component..."
$STIGCTL componentdefinition create Party '{
  "type": "object",
  "properties": {
    "pokemon": {
      "type": "array",
      "items": {"type": "string"},
      "maxItems": 6
    }
  },
  "required": ["pokemon"]
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

echo "  Creating Item component..."
$STIGCTL componentdefinition create Item '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "category": {"type": "string", "enum": ["pokeball", "potion", "status_heal", "battle_item", "tm", "key_item", "berry"]},
    "description": {"type": "string"},
    "effect": {"type": "string"}
  },
  "required": ["name", "category", "description", "effect"]
}'

echo "  Creating NPC component..."
$STIGCTL componentdefinition create NPC '{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "npc_type": {"type": "string", "enum": ["trainer", "gym_leader", "professor", "shopkeeper", "nurse", "generic"]},
    "dialogue": {
      "type": "array",
      "items": {"type": "string"}
    },
    "can_battle": {"type": "boolean"},
    "defeated": {"type": "boolean"}
  },
  "required": ["name", "npc_type", "dialogue", "can_battle", "defeated"]
}'

echo ""
echo "Creating entities and initial game state..."

echo "  Creating player entity..."
PLAYER_ENTITY=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Player entity: $PLAYER_ENTITY"

echo "  Creating starter pokemon entities..."
STARTER_1=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
STARTER_2=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
STARTER_3=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Starter 1 (Charmander): $STARTER_1"
echo "    Starter 2 (Squirtle): $STARTER_2"
echo "    Starter 3 (Bulbasaur): $STARTER_3"

echo "  Creating Professor Oak entity..."
PROF_OAK=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Professor Oak: $PROF_OAK"

echo "  Creating item entities..."
POKEBALL=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
POTION=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
ANTIDOTE=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Pokeball: $POKEBALL"
echo "    Potion: $POTION"
echo "    Antidote: $ANTIDOTE"

echo ""
echo "Attaching components to entities..."

echo "  Setting up player..."
$STIGCTL component create "$PLAYER_ENTITY" Trainer '{
  "name": "Ash",
  "money": 3000,
  "badges": [],
  "trainer_id": "trainer_000001"
}'

$STIGCTL component create "$PLAYER_ENTITY" Position '{
  "x": 10.0,
  "y": 10.0,
  "map": "pallet_town",
  "facing": "south"
}'

$STIGCTL component create "$PLAYER_ENTITY" Party '{
  "pokemon": []
}'

$STIGCTL component create "$PLAYER_ENTITY" Inventory '{
  "items": {
    "potion": 5,
    "pokeball": 10,
    "antidote": 3
  }
}'

echo "  Setting up Charmander (Fire starter)..."
$STIGCTL component create "$STARTER_1" PokemonSpecies '{
  "name": "Charmander",
  "pokedex_number": 4,
  "primary_type": "Fire",
  "secondary_type": null,
  "base_hp": 39,
  "base_attack": 52,
  "base_defense": 43,
  "base_sp_attack": 60,
  "base_sp_defense": 50,
  "base_speed": 65,
  "evolution_level": 16,
  "evolves_into": "Charmeleon"
}'

$STIGCTL component create "$STARTER_1" PokemonInstance '{
  "nickname": null,
  "level": 5,
  "current_hp": 20,
  "max_hp": 20,
  "attack": 11,
  "defense": 10,
  "sp_attack": 13,
  "sp_defense": 11,
  "speed": 14,
  "experience": 0,
  "experience_to_next_level": 125,
  "status": null,
  "friendship": 70
}'

$STIGCTL component create "$STARTER_1" MoveSet '{
  "moves": ["Scratch", "Growl"]
}'

echo "  Setting up Squirtle (Water starter)..."
$STIGCTL component create "$STARTER_2" PokemonSpecies '{
  "name": "Squirtle",
  "pokedex_number": 7,
  "primary_type": "Water",
  "secondary_type": null,
  "base_hp": 44,
  "base_attack": 48,
  "base_defense": 65,
  "base_sp_attack": 50,
  "base_sp_defense": 64,
  "base_speed": 43,
  "evolution_level": 16,
  "evolves_into": "Wartortle"
}'

$STIGCTL component create "$STARTER_2" PokemonInstance '{
  "nickname": null,
  "level": 5,
  "current_hp": 21,
  "max_hp": 21,
  "attack": 11,
  "defense": 14,
  "sp_attack": 11,
  "sp_defense": 14,
  "speed": 10,
  "experience": 0,
  "experience_to_next_level": 125,
  "status": null,
  "friendship": 70
}'

$STIGCTL component create "$STARTER_2" MoveSet '{
  "moves": ["Tackle", "Tail Whip"]
}'

echo "  Setting up Bulbasaur (Grass starter)..."
$STIGCTL component create "$STARTER_3" PokemonSpecies '{
  "name": "Bulbasaur",
  "pokedex_number": 1,
  "primary_type": "Grass",
  "secondary_type": "Poison",
  "base_hp": 45,
  "base_attack": 49,
  "base_defense": 49,
  "base_sp_attack": 65,
  "base_sp_defense": 65,
  "base_speed": 45,
  "evolution_level": 16,
  "evolves_into": "Ivysaur"
}'

$STIGCTL component create "$STARTER_3" PokemonInstance '{
  "nickname": null,
  "level": 5,
  "current_hp": 21,
  "max_hp": 21,
  "attack": 11,
  "defense": 11,
  "sp_attack": 14,
  "sp_defense": 14,
  "speed": 10,
  "experience": 0,
  "experience_to_next_level": 125,
  "status": null,
  "friendship": 70
}'

$STIGCTL component create "$STARTER_3" MoveSet '{
  "moves": ["Tackle", "Growl"]
}'

echo "  Setting up Professor Oak..."
$STIGCTL component create "$PROF_OAK" NPC '{
  "name": "Professor Oak",
  "npc_type": "professor",
  "dialogue": [
    "Welcome to the world of Pokemon!",
    "My name is Oak! People call me the Pokemon Prof!",
    "This world is inhabited by creatures called Pokemon!",
    "For some people, Pokemon are pets. Others use them for fights.",
    "Myself... I study Pokemon as a profession.",
    "Your very own Pokemon legend is about to unfold!",
    "A world of dreams and adventures with Pokemon awaits! Lets go!"
  ],
  "can_battle": false,
  "defeated": false
}'

$STIGCTL component create "$PROF_OAK" Position '{
  "x": 15.0,
  "y": 12.0,
  "map": "pallet_town",
  "facing": "south"
}'

echo "  Setting up items..."
$STIGCTL component create "$POKEBALL" Item '{
  "name": "Poke Ball",
  "category": "pokeball",
  "description": "A device for catching wild Pokemon",
  "effect": "catch_rate_1x"
}'

$STIGCTL component create "$POTION" Item '{
  "name": "Potion",
  "category": "potion",
  "description": "Restores 20 HP to a Pokemon",
  "effect": "heal_20"
}'

$STIGCTL component create "$ANTIDOTE" Item '{
  "name": "Antidote",
  "category": "status_heal",
  "description": "Cures a Pokemon of poison",
  "effect": "cure_poison"
}'

echo ""
echo "================================================"
echo "Pokemon World Setup Complete!"
echo "================================================"
echo ""
echo "Entity IDs:"
echo "  Player:          $PLAYER_ENTITY"
echo "  Charmander:      $STARTER_1"
echo "  Squirtle:        $STARTER_2"
echo "  Bulbasaur:       $STARTER_3"
echo "  Professor Oak:   $PROF_OAK"
echo "  Pokeball Item:   $POKEBALL"
echo "  Potion Item:     $POTION"
echo "  Antidote Item:   $ANTIDOTE"
echo ""
echo "To choose a starter Pokemon, add it to the player's party:"
echo "  $STIGCTL component update $PLAYER_ENTITY Party '{\"pokemon\": [\"$STARTER_1\"]}'"
echo ""
echo "To view all entities:"
echo "  $STIGCTL entity list"
echo ""
echo "To view player's components:"
echo "  $STIGCTL component list $PLAYER_ENTITY"
echo ""
