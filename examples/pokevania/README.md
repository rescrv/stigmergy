# Pokevania: The Power of ECS Composition

**A demonstration that merges Pokemon and Castlevania without writing a single new component definition.**

## Quick Links

- [QUICKSTART.md](QUICKSTART.md) - Get started in 5 minutes
- [POKEVANIA-DEMO.md](POKEVANIA-DEMO.md) - Full architectural documentation
- [SUMMARY.md](SUMMARY.md) - Technical achievement summary

## What Is This?

Pokevania demonstrates the core strength of Entity-Component Systems: **emergent gameplay through composition**. 

By running both the Pokemon and Castlevania setup scripts, you create a world where:
- Vampire hunters can catch Pokemon
- Pokemon can be gothic monsters
- Relics enhance Pokemon moves
- Gym leaders wield both whips and Pokeballs

**All without defining a single new component.**

## The Core Concept

```
Pokemon Setup:     Creates 9 component definitions
Castlevania Setup: Creates 9 component definitions
Pokevania Setup:   Creates 0 component definitions ← The magic!

Instead: Creates entities with components from BOTH systems
```

## Prerequisites

```bash
# 1. Start server
cargo run --bin stigmergyd

# 2. Setup Pokemon (creates Pokemon components)
cd examples/pokemon
./setup-pokemon-world.sh

# 3. Setup Castlevania (creates Castlevania components)
cd ../castelvania  
./setup-castelvania-world.sh

# 4. Now you're ready for Pokevania!
cd ../pokevania
```

## Usage

```bash
# Create hybrid entities
./setup-pokevania-world.sh

# Interactive exploration
./pokevania-examples.sh show-hybrid-trainer <entity-id>
./pokevania-examples.sh show-gothic-pokemon <entity-id>

# Full narrative experience
./pokevania-story.sh
```

## What Gets Created

### Hybrid Entities

1. **Julius Belmont** - Hunter + Trainer
   - Components: HunterProfile, HunterStats, Trainer, Party, Arsenal, Inventory, Location, QuestLog
   - Can hunt with whips AND catch with Pokeballs

2. **Gothic Gengar** - Pokemon + Monster  
   - Components: PokemonSpecies, PokemonInstance, MoveSet, MonsterProfile, MonsterState, Location
   - Is both a Ghost-type Pokemon and a nightmare-class monster

3. **Professor Alucard** - Mentor + NPC
   - Components: Mentor, NPC, Location
   - Teaches both hunting and training

4. **Crimson Moon Fragment** - Relic + Item
   - Components: Relic, Item, Location
   - Works in both systems

5. **Carmilla** - NPC + Trainer + Hunter
   - Triple hybrid: Gym leader who is also a fallen hunter
   - Has Pokemon team with gothic creatures

## Example Commands

```bash
# Create a new gothic pokemon
./pokevania-examples.sh create-gothic-pokemon misdreavus nightmare

# Weaken it with hunter combat
./pokevania-examples.sh gothic-battle <hunter-id> <pokemon-id>

# Catch it (uses whip + Dusk Ball)
./pokevania-examples.sh catch-gothic-pokemon <hunter-id> <pokemon-id>

# Enhance with relics
./pokevania-examples.sh equip-relic-to-pokemon <pokemon-id> "Crimson Moon Fragment"

# Use hunter healing on Pokemon
./pokevania-examples.sh use-moon-potion <hunter-id> <pokemon-id>

# Challenge hybrid gym leader
./pokevania-examples.sh challenge-gym <hunter-id> <leader-id>
```

## Files

```
pokevania/
├── README.md                   ← You are here
├── QUICKSTART.md               ← 5-minute start guide
├── POKEVANIA-DEMO.md           ← Full documentation
├── SUMMARY.md                  ← Technical summary
├── setup-pokevania-world.sh    ← Creates hybrid entities
├── pokevania-examples.sh       ← Interactive commands
└── pokevania-story.sh          ← Narrative demo
```

## The Philosophy

### Traditional OOP
```
class GothicPokemon extends Pokemon implements Monster {
    // Complex inheritance
    // Tight coupling
    // Rigid hierarchy
}
```

### ECS Approach
```bash
# Just compose!
entity + PokemonSpecies + MonsterProfile = Gothic Pokemon
entity + HunterProfile + Trainer = Hunter-Trainer
entity + Relic + Item = Hybrid item
```

## Why This Matters

1. **No New Code** - The server doesn't know about Pokevania
2. **Pure Composition** - Entities can have ANY component mix
3. **Emergent Gameplay** - New mechanics from component interactions
4. **Infinite Extensibility** - Add Metroid, Zelda, anything!

## Validation

Before running setup, validate prerequisites:

```bash
cd /cwd
./examples/test-pokevania-setup.sh
```

This checks that both Pokemon and Castlevania components exist.

## Demonstration Value

Pokevania proves that **ECS enables system composition at runtime**. You don't need:
- Inheritance hierarchies
- Multiple inheritance
- Interface implementations
- Code refactoring

You just need component definitions and entities that combine them.

## Next Steps

1. Run the setup scripts
2. Try the interactive commands
3. Watch the story unfold
4. Create your own hybrid entities
5. Imagine adding more game systems

## The Bottom Line

**Pokevania isn't a game. It's what emerges when two games share an ECS.**

That's composition. That's Stigmergy.

---

For questions or issues, see the main [examples README](../README.md).
