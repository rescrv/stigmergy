#!/usr/bin/env bash
set -euo pipefail

echo "Building stigctl..."
cargo build --bin stigctl
echo ""

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

echo "Verifying component definitions exist..."
echo "  Checking Pokemon components..."
if ! $STIGCTL componentdefinition list | grep -q "PokemonSpecies"; then
    echo "ERROR: Pokemon components not found. Please run examples/pokemon/setup-pokemon-world.sh first!"
    exit 1
fi

echo "  Checking Castlevania components..."
if ! $STIGCTL componentdefinition list | grep -q "HunterProfile"; then
    echo "ERROR: Castlevania components not found. Please run examples/castlevania/setup-castlevania-world.sh first!"
    exit 1
fi

echo "  All required components found!"
echo ""

echo "Creating Pokevania hybrid entities..."
echo ""

echo "  Creating hybrid hunter-trainer (Belmont with Pokemon)..."
BELMONT_TRAINER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Belmont Trainer: $BELMONT_TRAINER"

$STIGCTL component create "$BELMONT_TRAINER" HunterProfile '{
  "name": "Julius Belmont",
  "order": "Belmont Clan",
  "title": "Monster Tamer",
  "renown": 95,
  "vows": [
    "Tame the creatures of the night",
    "Turn darkness into strength",
    "Protect the realm with shadowed allies"
  ]
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" HunterStats '{
  "level": 15,
  "current_hp": 88,
  "max_hp": 105,
  "vitality": 25,
  "strength": 22,
  "focus": 28,
  "resilience": 24,
  "speed": 21,
  "resolve": 35,
  "status": "ready",
  "experience": 4800,
  "experience_to_next_rank": 1500
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" Trainer '{
  "name": "Julius Belmont",
  "money": 12000,
  "badges": ["Shadow Badge", "Crimson Badge", "Spectral Badge"],
  "trainer_id": "hunter_trainer_001"
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" Party '{
  "pokemon": []
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" Arsenal '{
  "primary_weapon": "Vampire Killer Whip",
  "backup_weapons": ["Dusk Pokeball Launcher", "Holy Boomerang"],
  "active_relics": ["Dark Amulet", "Creature Seal"]
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" Inventory '{
  "items": {
    "dusk_ball": 15,
    "moon_potion": 8,
    "holy_water": 5,
    "revive_crystal": 3,
    "shadow_flute": 1
  }
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" Location '{
  "region": "Transylvania",
  "area": "Castle Courtyard",
  "x": 22.0,
  "y": 15.0,
  "altitude": 580.0
}' > /dev/null

$STIGCTL component create "$BELMONT_TRAINER" QuestLog '{
  "active": [
    "Catch the legendary Darkrai haunting the clock tower",
    "Defeat the six Shadow Gym Leaders",
    "Restore the Moonlight Badge to Professor Alucard"
  ],
  "completed": [
    "Tame a Gengar from the catacombs",
    "Acquire the Dusk Ball prototype"
  ]
}' > /dev/null

echo "  Creating Gothic Gengar (Ghost Pokemon + Nightmare Monster)..."
GENGAR=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Gothic Gengar: $GENGAR"

$STIGCTL component create "$GENGAR" PokemonSpecies '{
  "name": "Gengar",
  "pokedex_number": 94,
  "primary_type": "Ghost",
  "secondary_type": "Poison",
  "base_hp": 60,
  "base_attack": 65,
  "base_defense": 60,
  "base_sp_attack": 130,
  "base_sp_defense": 75,
  "base_speed": 110,
  "evolution_level": null,
  "evolves_into": null
}' > /dev/null

$STIGCTL component create "$GENGAR" PokemonInstance '{
  "nickname": "Shadow Fiend",
  "level": 35,
  "current_hp": 145,
  "max_hp": 145,
  "attack": 95,
  "defense": 88,
  "sp_attack": 180,
  "sp_defense": 110,
  "speed": 152,
  "experience": 42875,
  "experience_to_next_level": 2125,
  "status": null,
  "friendship": 120
}' > /dev/null

$STIGCTL component create "$GENGAR" MoveSet '{
  "moves": ["Shadow Ball", "Sludge Bomb", "Dark Pulse", "Destiny Bond"]
}' > /dev/null

$STIGCTL component create "$GENGAR" MonsterProfile '{
  "name": "Shadow Fiend",
  "species": "Spectral Poisoner",
  "threat_level": "nightmare",
  "origin": "Manifested from castle catacombs",
  "weaknesses": ["Psychic sigils", "Moonlight", "Silver bells"],
  "lair": "Beneath the Chapel",
  "description": "A Pokemon twisted by centuries of dark energy, now a guardian of forbidden halls."
}' > /dev/null

$STIGCTL component create "$GENGAR" MonsterState '{
  "current_hp": 145,
  "max_hp": 145,
  "status": "lurking",
  "aggression": 45,
  "enrage": 10
}' > /dev/null

$STIGCTL component create "$GENGAR" Location '{
  "region": "Transylvania",
  "area": "Chapel Catacombs",
  "x": 8.0,
  "y": -12.0,
  "altitude": 180.0
}' > /dev/null

echo "  Creating Gothic Golbat (Flying Pokemon + Greater Threat)..."
GOLBAT=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Gothic Golbat: $GOLBAT"

$STIGCTL component create "$GOLBAT" PokemonSpecies '{
  "name": "Golbat",
  "pokedex_number": 42,
  "primary_type": "Poison",
  "secondary_type": "Flying",
  "base_hp": 75,
  "base_attack": 80,
  "base_defense": 70,
  "base_sp_attack": 65,
  "base_sp_defense": 75,
  "base_speed": 90,
  "evolution_level": null,
  "evolves_into": "Crobat"
}' > /dev/null

$STIGCTL component create "$GOLBAT" PokemonInstance '{
  "nickname": "Crimson Wing",
  "level": 28,
  "current_hp": 98,
  "max_hp": 98,
  "attack": 92,
  "defense": 82,
  "sp_attack": 78,
  "sp_defense": 88,
  "speed": 105,
  "experience": 21952,
  "experience_to_next_level": 1548,
  "status": null,
  "friendship": 85
}' > /dev/null

$STIGCTL component create "$GOLBAT" MoveSet '{
  "moves": ["Air Slash", "Poison Fang", "Bite", "Confuse Ray"]
}' > /dev/null

$STIGCTL component create "$GOLBAT" MonsterProfile '{
  "name": "Crimson Wing",
  "species": "Blood Drinker",
  "threat_level": "greater",
  "origin": "Spawned from castle belfry",
  "weaknesses": ["Sunlight", "Sacred crossbow", "Ice"],
  "lair": "Bell Tower Rafters",
  "description": "A swarm leader that feeds on the living, grown massive in the castle darkness."
}' > /dev/null

$STIGCTL component create "$GOLBAT" MonsterState '{
  "current_hp": 98,
  "max_hp": 98,
  "status": "lurking",
  "aggression": 70,
  "enrage": 30
}' > /dev/null

$STIGCTL component create "$GOLBAT" Location '{
  "region": "Transylvania",
  "area": "Bell Tower",
  "x": 45.0,
  "y": 72.0,
  "altitude": 1050.0
}' > /dev/null

echo "  Creating Gothic Haunter (Pre-evolution, Lesser Threat)..."
HAUNTER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Gothic Haunter: $HAUNTER"

$STIGCTL component create "$HAUNTER" PokemonSpecies '{
  "name": "Haunter",
  "pokedex_number": 93,
  "primary_type": "Ghost",
  "secondary_type": "Poison",
  "base_hp": 45,
  "base_attack": 50,
  "base_defense": 45,
  "base_sp_attack": 115,
  "base_sp_defense": 55,
  "base_speed": 95,
  "evolution_level": null,
  "evolves_into": "Gengar"
}' > /dev/null

$STIGCTL component create "$HAUNTER" PokemonInstance '{
  "nickname": null,
  "level": 18,
  "current_hp": 52,
  "max_hp": 52,
  "attack": 38,
  "defense": 35,
  "sp_attack": 78,
  "sp_defense": 42,
  "speed": 65,
  "experience": 5832,
  "experience_to_next_level": 768,
  "status": null,
  "friendship": 50
}' > /dev/null

$STIGCTL component create "$HAUNTER" MoveSet '{
  "moves": ["Lick", "Curse", "Night Shade"]
}' > /dev/null

$STIGCTL component create "$HAUNTER" MonsterProfile '{
  "name": "Wandering Shade",
  "species": "Lesser Phantom",
  "threat_level": "lesser",
  "origin": "Restless spirit from graveyard",
  "weaknesses": ["Holy symbols", "Light spells"],
  "lair": "Graveyard Mists",
  "description": "A young spirit learning to haunt, often found near tombstones."
}' > /dev/null

$STIGCTL component create "$HAUNTER" MonsterState '{
  "current_hp": 52,
  "max_hp": 52,
  "status": "dormant",
  "aggression": 25,
  "enrage": 0
}' > /dev/null

$STIGCTL component create "$HAUNTER" Location '{
  "region": "Transylvania",
  "area": "Castle Graveyard",
  "x": -5.0,
  "y": 8.0,
  "altitude": 420.0
}' > /dev/null

echo "  Creating Professor Alucard (Mentor + NPC hybrid)..."
ALUCARD=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Professor Alucard: $ALUCARD"

$STIGCTL component create "$ALUCARD" Mentor '{
  "name": "Professor Alucard",
  "role": "Master of Gothic Pokemon Studies",
  "dialogue": [
    "The creatures of night need not be our enemies.",
    "Through understanding, we tame the darkness itself.",
    "These Pokemon have adapted to feed on shadow energy.",
    "A true tamer wields both Pokeball and relic."
  ],
  "offers_training": true,
  "location_hint": "Research Laboratory in Castle West Wing"
}' > /dev/null

$STIGCTL component create "$ALUCARD" NPC '{
  "name": "Professor Alucard",
  "npc_type": "professor",
  "dialogue": [
    "Welcome, young tamer. I see the hunter blood in you.",
    "These Dusk Balls are specially designed for catching nocturnal Pokemon.",
    "Your next challenge awaits in the Spectral Gym.",
    "Remember: darkness is not evil, merely misunderstood."
  ],
  "can_battle": true,
  "defeated": false
}' > /dev/null

$STIGCTL component create "$ALUCARD" Location '{
  "region": "Transylvania",
  "area": "Research Laboratory",
  "x": -18.0,
  "y": 25.0,
  "altitude": 650.0
}' > /dev/null

echo "  Creating Crimson Moon Fragment (Relic + Item hybrid)..."
MOON_FRAG=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Crimson Moon Fragment: $MOON_FRAG"

$STIGCTL component create "$MOON_FRAG" Relic '{
  "name": "Crimson Moon Fragment",
  "category": "artifact",
  "description": "A shard of crystallized moonlight, tainted by ancient blood rituals",
  "power": "Empowers Dark and Ghost type moves by 30%"
}' > /dev/null

$STIGCTL component create "$MOON_FRAG" Item '{
  "name": "Crimson Moon Fragment",
  "category": "battle_item",
  "description": "When held by a Pokemon, boosts Ghost and Dark type moves",
  "effect": "ghost_dark_boost_30"
}' > /dev/null

$STIGCTL component create "$MOON_FRAG" Location '{
  "region": "Transylvania",
  "area": "Observatory Vault",
  "x": 52.0,
  "y": 94.0,
  "altitude": 1400.0
}' > /dev/null

echo "  Creating Dusk Ball (Item definition)..."
DUSK_BALL=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Dusk Ball: $DUSK_BALL"

$STIGCTL component create "$DUSK_BALL" Item '{
  "name": "Dusk Ball",
  "category": "pokeball",
  "description": "A specialized ball for catching Pokemon in dark places or at night",
  "effect": "catch_rate_3.5x_dark"
}' > /dev/null

echo "  Creating Shadow Gym Leader (NPC + Trainer hybrid)..."
SHADOW_LEADER=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
echo "    Shadow Gym Leader: $SHADOW_LEADER"

$STIGCTL component create "$SHADOW_LEADER" NPC '{
  "name": "Carmilla the Shadow Tamer",
  "npc_type": "gym_leader",
  "dialogue": [
    "So you dare enter my domain, little hunter?",
    "My Pokemon have feasted on centuries of darkness.",
    "You may carry the Belmont name, but can you handle true shadow?",
    "Impressive... Take this Shadow Badge. You have earned it."
  ],
  "can_battle": true,
  "defeated": false
}' > /dev/null

$STIGCTL component create "$SHADOW_LEADER" Trainer '{
  "name": "Carmilla",
  "money": 8800,
  "badges": [],
  "trainer_id": "gym_leader_shadow_001"
}' > /dev/null

$STIGCTL component create "$SHADOW_LEADER" Party '{
  "pokemon": []
}' > /dev/null

$STIGCTL component create "$SHADOW_LEADER" HunterProfile '{
  "name": "Carmilla",
  "order": "Fallen Order of Dawn",
  "title": "Shadow Tamer",
  "renown": 120,
  "vows": [
    "Command the night eternal",
    "Let no light pierce these halls"
  ]
}' > /dev/null

$STIGCTL component create "$SHADOW_LEADER" Location '{
  "region": "Transylvania",
  "area": "Shadow Gym - Throne Room",
  "x": 66.0,
  "y": 33.0,
  "altitude": 720.0
}' > /dev/null

echo ""
echo "================================================"
echo "Pokevania Hybrid World Setup Complete!"
echo "================================================"
echo ""
echo "Entity IDs:"
echo "  Belmont Trainer (Hunter+Trainer):  $BELMONT_TRAINER"
echo "  Gothic Gengar (Pokemon+Monster):   $GENGAR"
echo "  Gothic Golbat (Pokemon+Monster):   $GOLBAT"
echo "  Gothic Haunter (Pokemon+Monster):  $HAUNTER"
echo "  Professor Alucard (Mentor+NPC):    $ALUCARD"
echo "  Crimson Moon Fragment (Relic+Item): $MOON_FRAG"
echo "  Dusk Ball (Item):                  $DUSK_BALL"
echo "  Shadow Gym Leader (NPC+Trainer+Hunter): $SHADOW_LEADER"
echo ""
echo "Notice: Each entity has components from BOTH Pokemon and Castlevania systems!"
echo ""
echo "Try these commands:"
echo "  ./pokevania-examples.sh show-hybrid-trainer $BELMONT_TRAINER"
echo "  ./pokevania-examples.sh show-gothic-pokemon $GENGAR"
echo "  ./pokevania-examples.sh catch-gothic-pokemon $BELMONT_TRAINER $HAUNTER"
echo ""
echo "This demonstrates ECS composition: no new components needed,"
echo "just combining existing ones to create emergent gameplay!"
echo ""
