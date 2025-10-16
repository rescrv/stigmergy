# Stigmergy Examples

This directory contains example implementations demonstrating the power and flexibility of Stigmergy's Entity-Component System.

## Available Examples

### 1. Pokemon (`pokemon/`)
A Pokemon-like game world demonstrating classic ECS patterns for turn-based RPG mechanics.

**Key Components:**
- `PokemonSpecies`, `PokemonInstance`, `MoveSet`
- `Trainer`, `Party`, `Inventory`
- `Position`, `NPC`, `Item`

**Setup:**
```bash
cd pokemon
./setup-pokemon-world.sh
./pokemon-examples.sh list-entities
```

See [POKEMON-DEMO.md](pokemon/POKEMON-DEMO.md) for full documentation.

### 2. Castlevania (`castelvania/`)
A gothic action-adventure world inspired by Castlevania, showcasing different game mechanics and component design.

**Key Components:**
- `HunterProfile`, `HunterStats`, `Arsenal`
- `MonsterProfile`, `MonsterState`
- `Relic`, `QuestLog`, `Mentor`
- `Location` (similar to but distinct from Pokemon's `Position`)

**Setup:**
```bash
cd castelvania
./setup-castelvania-world.sh
./castelvania-examples.sh list-entities
```

See [CASTELVANIA-DEMO.md](castelvania/CASTELVANIA-DEMO.md) for full documentation.

### 3. Pokevania (`pokevania/`) ⭐ **Demonstrates ECS Composition Power**
A hybrid world that merges Pokemon and Castlevania mechanics WITHOUT requiring any new component definitions. This example showcases the true power of Entity-Component Systems: **emergent gameplay through component composition**.

**What Makes This Special:**
- **No new components needed** - Uses only components from Pokemon and Castlevania
- **Hybrid entities** - Single entities with components from both systems
- **Emergent behavior** - New gameplay emerges from combining existing components
- **Zero code changes** - The server doesn't know about "Pokevania"

**Hybrid Entity Examples:**
- **Hunter-Trainers**: Entities with `HunterProfile` + `Trainer` + `Arsenal` + `Party`
- **Gothic Pokemon**: Entities with `PokemonSpecies` + `MonsterProfile` + both stat systems
- **Dual NPCs**: Mentors who are also Pokemon professors
- **Hybrid Items**: Relics that function as Pokemon items

**Setup (requires both Pokemon and Castlevania components first):**
```bash
# 1. Setup Pokemon world (creates Pokemon component definitions)
cd pokemon
./setup-pokemon-world.sh

# 2. Setup Castlevania world (creates Castlevania component definitions)
cd ../castelvania
./setup-castelvania-world.sh

# 3. Create hybrid Pokevania entities (uses components from both)
cd ../pokevania
./setup-pokevania-world.sh
```

**Try it out:**
```bash
# Show a hunter who is also a Pokemon trainer
./pokevania-examples.sh show-hybrid-trainer <entity-id>

# Show a Pokemon that is also a gothic monster
./pokevania-examples.sh show-gothic-pokemon <entity-id>

# Experience the full narrative
./pokevania-story.sh
```

See [POKEVANIA-DEMO.md](pokevania/POKEVANIA-DEMO.md) for full documentation on how ECS composition creates emergent gameplay.

## The Progression

These examples are designed to be explored in order:

1. **Pokemon** - Learn basic ECS patterns
2. **Castlevania** - See alternative component designs for different mechanics
3. **Pokevania** - Understand the power of composition when both systems coexist

## Architecture Demonstration

### Traditional OOP Approach (What We Avoid)
```
class Pokemon { ... }
class Monster { ... }

// Need new code to merge them
class GothicPokemon extends Pokemon implements Monster {
    // Complex inheritance, tight coupling
}
```

### ECS Approach (What Stigmergy Enables)
```bash
# Create entity
ENTITY=$(stigctl entity create)

# Add Pokemon components
stigctl component create $ENTITY PokemonSpecies '{...}'
stigctl component create $ENTITY PokemonInstance '{...}'

# Add Monster components to THE SAME ENTITY
stigctl component create $ENTITY MonsterProfile '{...}'
stigctl component create $ENTITY MonsterState '{...}'

# Now it's both! No new code needed.
```

## Running the Examples

### Prerequisites
1. Start the Stigmergy server:
   ```bash
   cargo run --bin stigmergyd
   ```

2. The server runs on `http://localhost:8080` by default

### Common Operations

All examples support similar operations via their `*-examples.sh` scripts:
- `list-entities` - See all entities
- `show-*` - Display entity details
- `create-*` - Make new entities
- Various gameplay commands specific to each world

### Interactive Stories

Each example includes a story script that creates entities and demonstrates gameplay:
- `pokemon-story.sh` - Journey to become a Pokemon master
- `castelvania-story.sh` - Hunt monsters in a gothic castle
- `pokevania-story.sh` - Experience the fusion of both worlds

## Key Takeaways

1. **Composition over Inheritance** - Mix components freely without class hierarchies
2. **Data-Driven Design** - Entities are just IDs; components are pure data
3. **Emergent Behavior** - New gameplay emerges from component combinations
4. **Zero Coupling** - Systems don't need to know about each other
5. **Query Flexibility** - Find entities by any component combination

## Files Overview

Each example directory contains:
- `setup-*-world.sh` - Creates component definitions and initial entities
- `*-examples.sh` - Interactive command interface
- `*-story.sh` - Narrative demonstration
- `*-DEMO.md` - Full documentation

## Next Steps

After exploring these examples, try:
1. Mix in your own components
2. Create entities with novel component combinations
3. Build systems that query entities by component patterns
4. Experiment with other game genres using the same principles
