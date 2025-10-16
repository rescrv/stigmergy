# Pokevania: The Power of ECS Composition

This demo showcases the true power of Entity-Component Systems: **emergent gameplay through component composition**. By running both the Pokemon and Castlevania setup scripts, we create a hybrid world where entities can freely mix components from both systems.

## The Magic of ECS

Traditional object-oriented inheritance would require creating new classes:
```
class PokemonHunter extends Hunter implements PokemonTrainer { ... }
class GothicPokemon extends Pokemon implements Monster { ... }
```

With ECS, we simply attach components:
- Entity + `Trainer` + `HunterProfile` = A trainer who is also a vampire hunter
- Entity + `PokemonSpecies` + `MonsterProfile` = A Pokemon that is also a gothic monster
- Entity + `Arsenal` + `Party` = A hunter with both weapons AND Pokemon

## Setup

### 1. Start the Stigmergy Server
```bash
cargo run --bin stigmergyd
```

### 2. Setup Both Worlds
```bash
cd examples/pokemon
./setup-pokemon-world.sh

cd ../castelvania
./setup-castelvania-world.sh
```

Now both component definitions exist in the system!

### 3. Create the Pokevania Hybrid World
```bash
cd ../pokevania
./setup-pokevania-world.sh
```

This creates hybrid entities that use components from BOTH systems:
- **Monster Trainers**: Entities with `HunterProfile` + `Trainer` + `Arsenal` + `Party`
- **Gothic Pokemon**: Entities with `PokemonSpecies` + `PokemonInstance` + `MonsterProfile` + `MonsterState`
- **Hybrid NPCs**: Mentors who are also Pokemon professors
- **Relic Items**: Items that are both Pokemon items and Castlevania relics

## Hybrid Entities

### Vampire Hunter Trainer
An entity that has:
- `HunterProfile`: Name, order (Belmont Clan), vows
- `HunterStats`: HP, combat stats, status
- `Trainer`: Money, badges, trainer ID
- `Party`: Pokemon team (max 6)
- `Arsenal`: Vampire Killer whip + relics
- `Inventory`: Mix of potions, pokeballs, holy water
- `Position`/`Location`: Works in both systems!

### Gothic Pokemon (e.g., Gengar the Nightmare)
An entity that has:
- `PokemonSpecies`: Ghost/Poison type, base stats
- `PokemonInstance`: Level, HP, moves, status
- `MonsterProfile`: Threat level "nightmare", weaknesses
- `MonsterState`: Aggression, enrage status
- `MoveSet`: Both Pokemon moves AND monster abilities

### Dark Gym Leader
An entity that has:
- `NPC`: Dialogue, can_battle
- `HunterProfile`: A fallen hunter turned gym leader
- `Party`: Team of Gothic Pokemon
- `Location`: Guards a castle region

## Commands

```bash
# List all entities (shows Pokemon, Castlevania, and hybrid entities)
./pokevania-examples.sh list-entities

# Show a hybrid trainer (both hunter and pokemon trainer aspects)
./pokevania-examples.sh show-hybrid-trainer <entity-id>

# Show a gothic pokemon (both pokemon and monster aspects)
./pokevania-examples.sh show-gothic-pokemon <entity-id>

# Equip a relic to enhance pokemon moves
./pokevania-examples.sh equip-relic-to-pokemon <pokemon-id> "Solar Sigil"

# Battle with both hunter combat and pokemon mechanics
./pokevania-examples.sh gothic-battle <trainer-id> <gothic-pokemon-id>

# Catch a wild monster-pokemon
./pokevania-examples.sh catch-gothic-pokemon <hunter-trainer-id> <wild-id>

# Use hunter healing on pokemon
./pokevania-examples.sh use-healing-vial <trainer-id> <pokemon-id>

# Create new gothic pokemon
./pokevania-examples.sh create-gothic-pokemon <species> <threat-level>
```

## Example Workflow

```bash
# 1. Start server and setup both worlds
cargo run --bin stigmergyd &
cd examples/pokemon && ./setup-pokemon-world.sh
cd ../castelvania && ./setup-castelvania-world.sh
cd ../pokevania && ./setup-pokevania-world.sh

# 2. Get your hybrid hunter-trainer
HUNTER_TRAINER=$(./target/debug/stigctl entity list | grep entity: | head -1 | awk '{print $2}')

# 3. View their dual nature
./pokevania-examples.sh show-hybrid-trainer "$HUNTER_TRAINER"

# 4. Create a Gothic Gengar
GENGAR=$(./pokevania-examples.sh create-gothic-pokemon Gengar nightmare | grep "Pokemon ID:" | awk '{print $3}')

# 5. Battle and catch it
./pokevania-examples.sh gothic-battle "$HUNTER_TRAINER" "$GENGAR"
./pokevania-examples.sh catch-gothic-pokemon "$HUNTER_TRAINER" "$GENGAR"

# 6. View your new team member
./pokevania-examples.sh show-gothic-pokemon "$GENGAR"

# 7. Equip a relic to boost shadow moves
./pokevania-examples.sh equip-relic-to-pokemon "$GENGAR" "Crimson Moon Fragment"
```

## The ECS Advantage

### Composition Over Inheritance
Instead of rigid class hierarchies, we compose behavior:
- Want a Pokemon that's also a monster? Add both component types.
- Want a trainer who's also a hunter? Add both profiles.
- Want an item that works in both systems? Attach both component definitions.

### No Code Changes Required
The Stigmergy server doesn't know or care about "Pokevania". It just stores and retrieves components. The hybrid behavior emerges naturally from having both component definitions available.

### Query Flexibility
Systems can query entities by component combinations:
- "All entities with `PokemonInstance` AND `MonsterState`" = Gothic Pokemon
- "All entities with `Trainer` AND `HunterProfile`" = Hybrid trainers
- "All entities with `Party` AND `Arsenal`" = Battle-ready hybrid characters

### Easy Extension
Add more systems easily:
- Add Metroid components → Pokevania meets space bounty hunting
- Add Zelda components → Pokevania meets dungeon crawling
- No refactoring needed, just more component definitions!

## Architectural Notes

### Shared Components
Some components work naturally in both systems:
- `Location` (from Castlevania) can replace `Position` (from Pokemon)
- `Inventory` exists in both with identical schema
- Both use entity references for relationships

### Complementary Components
Components enhance each other:
- `Arsenal.active_relics` can buff `PokemonInstance.attack`
- `MonsterProfile.weaknesses` adds depth to `PokemonSpecies.type`
- `HunterStats.resolve` could affect `Trainer.money` management

### Emergent Gameplay
New mechanics emerge from composition:
- Pokemon with high `MonsterState.aggression` are harder to catch
- Hunters with `Arsenal` can weaken monsters before catching
- `Relic` components can modify Pokemon move effectiveness
- `QuestLog` quests can involve both hunting and catching

## Files

- `setup-pokevania-world.sh`: Creates hybrid entities (requires both pokemon and castlevania components)
- `pokevania-examples.sh`: Interactive commands for the hybrid world
- `pokevania-story.sh`: Narrative demo showing composition in action
- `POKEVANIA-DEMO.md`: This documentation

## Requirements

This demo requires that you first run:
1. `examples/pokemon/setup-pokemon-world.sh` - Creates Pokemon component definitions
2. `examples/castelvania/setup-castelvania-world.sh` - Creates Castlevania component definitions

Then run `setup-pokevania-world.sh` to create entities that use components from BOTH systems.

## Notes

- Entity IDs are shared across all systems
- Components can be mixed freely on any entity
- No special "hybrid" components needed - composition is the feature!
- The server doesn't distinguish between Pokemon, Castlevania, or Pokevania entities
- All operations work with standard `stigctl` commands
