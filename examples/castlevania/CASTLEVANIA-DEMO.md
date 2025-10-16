# Castlevania-Inspired World Demo for Stigmergy

This demo explores how to model a gothic, Castlevania-flavored adventure using Stigmergy's Entity-Component System. It mirrors the Pokemon example structure while introducing new components suited to vampire hunters, relics, and night-bound horrors.

## Overview

The demo includes:
- **Component Definitions** tailored for hunters, relics, monsters, mentors, and quests
- **Entities** representing a prepared hunter, a mentor NPC, a nightmare-class monster, and a roaming relic
- **Interactive Scripts** to set up the world, drive a narrative encounter, and perform common operations with `stigctl`

## Components

### Hunter Components
- **HunterProfile**: Identity, order affiliation, title, renown, vows
- **HunterStats**: Combat readiness metrics and advancement info
- **Arsenal**: Primary weapon, backup arms, and active relics
- **Inventory**: Flexible bag of consumables and tools
- **QuestLog**: Active and completed objectives

### Monster Components
- **MonsterProfile**: Species, threat level, origin, weaknesses, lair
- **MonsterState**: Health, aggression, enrage, and current status

### World & Support Components
- **Location**: Spatial placement within the castle regions
- **Relic**: Artifact metadata and special power description
- **Mentor**: Narrative NPC supplying lore or training cues

## Getting Started

1. **Start the Stigmergy server**
   ```bash
   cargo run --bin stigmergyd
   ```
   By default the server listens on `http://localhost:8080`.

2. **Setup the Castlevania world**
   ```bash
   ./setup-castlevania-world.sh
   ```
   The script builds `stigctl` if needed, registers all component definitions, and seeds:
   - Richter Belmont-like hunter entity with stats, arsenal, quests
   - Archivist mentor NPC
   - Nightmare-class monster guarding a relic
   - A Solar Sigil relic entity

3. **Experience the narrative vignette (optional but fun)**
   ```bash
   ./castlevania-story.sh
   ```
   This script creates a fresh protagonist, equips relics, and stages a multi-round banishment encounter complete with dramatic pacing.

4. **Interact via the examples helper**
   ```bash
   ./castlevania-examples.sh list-entities
   ./castlevania-examples.sh show-hunter <hunter-entity-id>
   ./castlevania-examples.sh equip-relic <hunter-entity-id> "Solar Sigil"
   ./castlevania-examples.sh create-monster "Bone Golem" greater
   ```

## Key Commands in `castlevania-examples.sh`

- `list-entities`: Inspect all entities currently stored
- `show-hunter`: Display HunterProfile, HunterStats, Arsenal, Inventory, QuestLog
- `use-vial`: Spend a healing vial to restore health (fails loudly if none remain)
- `record-quest` / `complete-quest`: Manage quest progression safely using `jq`
- `equip-relic` / `collect-relic`: Add relics to the hunter's arsenal by name or entity lookup
- `create-monster`: Spawn templated monsters with themed stats at varying threat levels
- `banish-monster`: Force a monster's state to a banished status for rapid iteration

## Manual Exploration with `stigctl`

All definitions and entities are standard Stigmergy resources, so you can mix direct CLI usage:
```bash
./target/debug/stigctl componentdefinition list
./target/debug/stigctl component list <entity-id>
./target/debug/stigctl component get <entity-id> HunterStats
./target/debug/stigctl component update <entity-id> QuestLog '{"active":[],"completed":[]}'
```

## Extending the Demo

- **Add hunters**: Duplicate the setup pattern to create rival slayers with different arsenals
- **Design relic hunts**: Introduce additional Relic entities with Location trails and use the helper script to collect them
- **Broaden monsters**: Expand `create_monster` cases or author new threat presets to simulate raids
- **Automate encounters**: Compose shell or Rust scripts that orchestrate `stigctl` commands for battles, sanctum defenses, or castle shifts

## Files

- `setup-castlevania-world.sh`: Provision component definitions and seed entities
- `castlevania-examples.sh`: Utility CLI for day-to-day interactions
- `castlevania-story.sh`: Narrative showcase with staged combat beats
- `CASTELVANIA-DEMO.md`: This documentation guide

## Notes

- All scripts assume the server runs at `http://localhost:8080`; override with `BASE_URL` if needed
- Ensure `jq` is installed—the helper scripts rely on it for JSON manipulation
- Entity IDs printed during setup can be reused across scripts and manual CLI calls
