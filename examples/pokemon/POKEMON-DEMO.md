# Pokemon-like Game Demo for Stigmergy

This demo showcases how to use Stigmergy's Entity-Component System to build a Pokemon-like game world.

## Overview

The demo includes:
- **Component Definitions**: Types that define the schema for different game elements
- **Entities**: Individual game objects (player, pokemon, NPCs, items)
- **Component Instances**: Actual data attached to entities

## Component Types

### Pokemon Components
- **PokemonSpecies**: Static species data (name, type, base stats, evolution)
- **PokemonInstance**: Individual pokemon data (level, current HP, stats, status, XP)
- **Move**: Individual move definitions
- **MoveSet**: List of moves a pokemon knows (max 4)

### Trainer Components
- **Trainer**: Trainer profile (name, money, badges, ID)
- **Party**: Pokemon in the trainer's active party (max 6)
- **Inventory**: Items the trainer owns

### World Components
- **Position**: Location in the game world (x, y, map, facing direction)
- **NPC**: Non-player character data
- **Item**: Item definitions (pokeballs, potions, etc.)

## Getting Started

### 1. Start the Stigmergy Server

```bash
cargo run --bin stigmergyd
```

The server will start on `http://localhost:8080` by default.

### 2. Run the Setup Script

```bash
./setup-pokemon-world.sh
```

The script will automatically build `stigctl` if needed, then create:
- All component definitions
- A player entity with trainer, position, party, and inventory
- Three starter pokemon (Charmander, Squirtle, Bulbasaur)
- Professor Oak NPC
- Basic items (Pokeball, Potion, Antidote)

The script will output entity IDs for all created entities.

### 3. Interact with the World

Use the examples script to perform common operations:

```bash
# List all entities
./pokemon-examples.sh list-entities

# View player details
./pokemon-examples.sh show-trainer <player-entity-id>

# View pokemon details
./pokemon-examples.sh show-pokemon <pokemon-entity-id>

# Add a starter to the player's party
./pokemon-examples.sh add-to-party <player-id> <pokemon-id>

# Level up a pokemon
./pokemon-examples.sh level-up <pokemon-id>

# Catch a wild Rattata
./pokemon-examples.sh catch-pokemon <player-id>

# Create a wild Pikachu
./pokemon-examples.sh create-wild-pokemon pikachu

# Apply battle damage
./pokemon-examples.sh battle-damage <pokemon-id> 15

# Use a potion
./pokemon-examples.sh use-potion <player-id> <pokemon-id>

# Heal a pokemon completely
./pokemon-examples.sh heal-pokemon <pokemon-id>
```

## Manual Operations with stigctl

You can also use `stigctl` directly:

```bash
# List component definitions
./target/debug/stigctl componentdefinition list

# Get a specific component
./target/debug/stigctl component get <entity-id> PokemonSpecies

# Update a component
./target/debug/stigctl component update <entity-id> PokemonInstance '{"level": 10, ...}'

# Create a new entity
./target/debug/stigctl entity create

# Delete an entity
./target/debug/stigctl entity delete <entity-id>
```

## Example Workflow

```bash
# 1. Start the server
cargo run --bin stigmergyd &

# 2. Setup the world (builds stigctl automatically)
./setup-pokemon-world.sh

# 3. Save the player entity ID
PLAYER=$(./target/debug/stigctl entity list | grep entity: | head -1 | awk '{print $2}')

# 4. Choose Charmander as starter (use the ID from setup output)
CHARMANDER="entity:XXXXXXXXXX"
./pokemon-examples.sh add-to-party "$PLAYER" "$CHARMANDER"

# 5. View your party
./target/debug/stigctl component get "$PLAYER" Party

# 6. View Charmander's stats
./pokemon-examples.sh show-pokemon "$CHARMANDER"

# 7. Battle! Apply some damage
./pokemon-examples.sh battle-damage "$CHARMANDER" 12

# 8. Heal with a potion
./pokemon-examples.sh use-potion "$PLAYER" "$CHARMANDER"

# 9. Level up after winning
./pokemon-examples.sh level-up "$CHARMANDER"

# 10. Catch a Pikachu
PIKACHU=$(./pokemon-examples.sh create-wild-pokemon pikachu | grep "Pokemon ID:" | awk '{print $3}')

# 11. Add to party
./pokemon-examples.sh add-to-party "$PLAYER" "$PIKACHU"
```

## Architecture Notes

### Entity-Component System (ECS)

This demo uses a pure ECS architecture:
- **Entities** are just IDs (containers)
- **Components** are pure data (no logic)
- **Systems** (not included in this demo) would process entities with specific component combinations

### Composition over Inheritance

Instead of `class WildPokemon extends Pokemon`, we use:
- Entity with `PokemonSpecies` + `PokemonInstance` + `MoveSet` = A pokemon
- Entity with above + `Position` = Wild pokemon in the world
- Entity with `Trainer` + `Party` = A trainer
- Entity with `Trainer` + `Party` + `Position` = Trainer in the world

### Benefits

1. **Flexibility**: Mix and match components freely
2. **Data-driven**: Easy to serialize/deserialize
3. **Query-friendly**: Find all entities with specific components
4. **Extensible**: Add new components without changing existing code

## Extending the Demo

### Add New Pokemon

Create a function in `pokemon-examples.sh` following the pattern:

```bash
create_pokemon_magikarp() {
    local entity=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
    $STIGCTL component create "$entity" PokemonSpecies '{...}'
    $STIGCTL component create "$entity" PokemonInstance '{...}'
    $STIGCTL component create "$entity" MoveSet '{...}'
    echo "$entity"
}
```

### Add New Component Types

```bash
# Example: Add a StatusEffect component
./target/debug/stigctl componentdefinition create StatusEffect '{
  "type": "object",
  "properties": {
    "effect": {"type": "string"},
    "turns_remaining": {"type": "integer"},
    "damage_per_turn": {"oneOf": [{"type": "integer"}, {"type": "null"}]}
  },
  "required": ["effect", "turns_remaining"]
}'
```

### Create Game Systems

Write systems that:
1. Query entities with specific components
2. Process/update their data
3. Handle game logic (battles, evolution, movement, etc.)

Example system concepts:
- **BattleSystem**: Processes entities with `PokemonInstance` + `MoveSet`
- **EvolutionSystem**: Checks level and evolves pokemon
- **MovementSystem**: Updates `Position` component
- **NPCSystem**: Handles dialogue and interactions

## Files

- `setup-pokemon-world.sh`: Initial world setup script
- `pokemon-examples.sh`: Interactive examples and utilities
- `POKEMON-DEMO.md`: This documentation file

## Notes

- Entity IDs are generated as base64-encoded strings
- All component data must match the JSON schema defined in component definitions
- The server stores everything in PostgreSQL (after migrations are run)
- Component updates are PUT operations (full replacement)
