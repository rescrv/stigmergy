# Pokevania Quick Start Guide

## What is Pokevania?

Pokevania demonstrates the **power of Entity-Component System (ECS) composition** by merging Pokemon and Castlevania gameplay WITHOUT writing any new component definitions. It's pure composition!

## The Key Insight

```
Traditional OOP: PokemonHunter extends Pokemon implements Hunter
                 ↓
             Complex inheritance, tight coupling

ECS Approach:    Entity + PokemonSpecies + HunterProfile
                 ↓
             Simple composition, zero coupling
```

## Prerequisites

You MUST run both setup scripts first to create the component definitions:

```bash
# Terminal 1: Start server
cargo run --bin stigmergyd

# Terminal 2: Setup both worlds
cd examples/pokemon
./setup-pokemon-world.sh

cd ../castelvania
./setup-castelvania-world.sh
```

## Quick Start

```bash
cd examples/pokevania

# Setup hybrid entities
./setup-pokevania-world.sh

# View a hybrid trainer (both hunter AND pokemon trainer)
./pokevania-examples.sh show-hybrid-trainer <entity-id>

# View a gothic pokemon (both pokemon AND monster)
./pokevania-examples.sh show-gothic-pokemon <entity-id>

# Experience the full story
./pokevania-story.sh
```

## What Gets Created

### Hybrid Entities

1. **Julius Belmont (Hunter + Trainer)**
   - Has: `HunterProfile`, `HunterStats`, `Trainer`, `Party`, `Arsenal`, `Inventory`, `Location`, `QuestLog`
   - Can: Hunt monsters, catch Pokemon, wield relics, earn badges

2. **Gothic Gengar (Pokemon + Monster)**
   - Has: `PokemonSpecies`, `PokemonInstance`, `MoveSet`, `MonsterProfile`, `MonsterState`, `Location`
   - Is: Both a Ghost-type Pokemon AND a nightmare-class monster

3. **Professor Alucard (Mentor + NPC)**
   - Has: `Mentor`, `NPC`, `Location`
   - Can: Train hunters, give Pokemon, offer quests

4. **Crimson Moon Fragment (Relic + Item)**
   - Has: `Relic`, `Item`, `Location`
   - Works: In both hunter arsenal and Pokemon held items

## Example Commands

```bash
# Create a new gothic pokemon
./pokevania-examples.sh create-gothic-pokemon misdreavus nightmare

# Battle and weaken it (uses hunter combat + pokemon stats)
./pokevania-examples.sh gothic-battle <hunter-id> <pokemon-id>

# Catch it (uses both Dusk Ball AND Vampire Killer whip)
./pokevania-examples.sh catch-gothic-pokemon <hunter-id> <pokemon-id>

# Equip a relic to boost its shadow moves
./pokevania-examples.sh equip-relic-to-pokemon <pokemon-id> "Crimson Moon Fragment"

# Use hunter healing on Pokemon
./pokevania-examples.sh use-moon-potion <hunter-id> <pokemon-id>

# Challenge a gym leader (who is also a fallen hunter)
./pokevania-examples.sh challenge-gym <hunter-id> <leader-id>
```

## The Magic of Composition

### What Makes This Possible?

1. **No New Code**: The Stigmergy server doesn't know about "Pokevania"
2. **No New Components**: Uses only Pokemon and Castlevania components
3. **Pure Composition**: Entities can have ANY combination of components
4. **Emergent Gameplay**: New mechanics emerge from component interactions

### Example: Gothic Battle

When you run `gothic-battle`:
- Uses `HunterStats.strength` for attack damage
- Reads `MonsterState.aggression` for counter-attack
- Updates BOTH `HunterStats.current_hp` AND `MonsterState.current_hp`
- Two systems working together through shared entity!

### Example: Catch Gothic Pokemon

When you run `catch-gothic-pokemon`:
- Checks `MonsterState.aggression` (harder to catch if high)
- Uses `Inventory.dusk_ball` (from Pokemon system)
- Applies `Arsenal` weapons to weaken (from Castlevania system)
- Adds to `Party.pokemon` array (from Pokemon system)
- Sets `MonsterState.status = "dormant"` (from Castlevania system)

## Composition Patterns

```bash
# Pattern 1: Shared Spatial Data
Location (Castlevania) ≈ Position (Pokemon)
Both can coexist! Location is more detailed (region, area, altitude)

# Pattern 2: Dual Stat Systems
PokemonInstance.current_hp ←→ MonsterState.current_hp
Keep them in sync for hybrid entities!

# Pattern 3: Dual Identity
Trainer.trainer_id = "hunter_001"
HunterProfile.title = "Monster Tamer"
Same entity, two identities!

# Pattern 4: Enhanced Items
Relic.power = "Boosts Ghost moves"
Item.effect = "ghost_dark_boost_30"
Same object, works in both systems!
```

## Philosophy

### ECS Enables

✓ Mix components from different "games" freely  
✓ No inheritance hierarchies to maintain  
✓ Add new systems without modifying existing ones  
✓ Query entities by any component combination  
✓ Pure data, easy to serialize/deserialize  

### Traditional OOP Prevents

✗ Rigid class hierarchies  
✗ Multiple inheritance problems  
✗ God objects trying to be everything  
✗ Tight coupling between systems  
✗ Can't extend without modifying source  

## Experimentation Ideas

Try creating your own hybrid entities:

```bash
# A Pokemon that's also a relic
stigctl entity create  # Get entity ID
stigctl component create $ENTITY PokemonSpecies '{...}'
stigctl component create $ENTITY Relic '{...}'
# Now it's a mystical creature that IS a relic!

# A trainer who's also a monster
stigctl component create $ENTITY Trainer '{...}'
stigctl component create $ENTITY MonsterProfile '{...}'
# A cursed trainer transforming into a monster!

# An NPC who can battle and offers quests
stigctl component create $ENTITY NPC '{...}'
stigctl component create $ENTITY Party '{...}'
stigctl component create $ENTITY QuestLog '{...}'
# A questgiver with a Pokemon team!
```

## Troubleshooting

### "Component definition not found"
→ You need to run the Pokemon and Castlevania setup scripts first

### "Entity not found"
→ Run `./setup-pokevania-world.sh` to create hybrid entities

### "Server connection refused"
→ Start the server: `cargo run --bin stigmergyd`

## Next Steps

1. Read [POKEVANIA-DEMO.md](POKEVANIA-DEMO.md) for architectural deep dive
2. Study the scripts to see how components are combined
3. Create your own hybrid entities with novel component mixes
4. Build systems that query for specific component combinations
5. Imagine what happens when you add a THIRD system (Metroid? Zelda?)

## The Bottom Line

**Pokevania isn't a separate game—it's what emerges when Pokemon and Castlevania components coexist in the same ECS.**

That's the power of composition. That's the power of Stigmergy.
