#!/usr/bin/env bash
set -euo pipefail

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

EXAMPLES="./examples/pokevania/pokevania-examples.sh"

echo "========================================"
echo "  POKEVANIA: A TALE OF TWO SYSTEMS"
echo "========================================"
echo ""
sleep 2

echo "In a world where Pokemon and monsters are one..."
echo "Where trainers wield both Pokeball and whip..."
echo "A young Belmont discovers the true power of composition."
echo ""
sleep 3

echo "--- ACT I: THE AWAKENING ---"
echo ""
sleep 1

echo "Creating our hero: Marcus Belmont, a hunter-in-training..."
MARCUS=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$MARCUS" HunterProfile '{
  "name": "Marcus Belmont",
  "order": "Belmont Clan",
  "title": "Apprentice Tamer",
  "renown": 15,
  "vows": ["Learn the old ways", "Prove my worth"]
}' > /dev/null

$STIGCTL component create "$MARCUS" HunterStats '{
  "level": 3,
  "current_hp": 45,
  "max_hp": 45,
  "vitality": 12,
  "strength": 10,
  "focus": 14,
  "resilience": 11,
  "speed": 13,
  "resolve": 15,
  "status": "ready",
  "experience": 250,
  "experience_to_next_rank": 750
}' > /dev/null

$STIGCTL component create "$MARCUS" Trainer '{
  "name": "Marcus Belmont",
  "money": 2000,
  "badges": [],
  "trainer_id": "apprentice_001"
}' > /dev/null

$STIGCTL component create "$MARCUS" Party '{
  "pokemon": []
}' > /dev/null

$STIGCTL component create "$MARCUS" Arsenal '{
  "primary_weapon": "Training Whip",
  "backup_weapons": ["Wooden Stake"],
  "active_relics": []
}' > /dev/null

$STIGCTL component create "$MARCUS" Inventory '{
  "items": {
    "dusk_ball": 5,
    "moon_potion": 3,
    "training_manual": 1
  }
}' > /dev/null

$STIGCTL component create "$MARCUS" Location '{
  "region": "Transylvania",
  "area": "Training Grounds",
  "x": 0.0,
  "y": 0.0,
  "altitude": 450.0
}' > /dev/null

echo "Marcus Belmont created!"
echo "Entity ID: $MARCUS"
sleep 2
echo ""

echo "Marcus approaches his mentor, Professor Alucard..."
echo ""
sleep 2

ALUCARD=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$ALUCARD" Mentor '{
  "name": "Professor Alucard",
  "role": "Master of Gothic Pokemon",
  "dialogue": [
    "Young Belmont, your training begins now.",
    "The creatures of night need not be our enemies.",
    "With understanding comes power."
  ],
  "offers_training": true,
  "location_hint": "Training Grounds"
}' > /dev/null

$STIGCTL component create "$ALUCARD" NPC '{
  "name": "Professor Alucard",
  "npc_type": "professor",
  "dialogue": [
    "Marcus, you carry the blood of hunters, but also the heart of a tamer.",
    "Today, you will catch your first Gothic Pokemon."
  ],
  "can_battle": false,
  "defeated": false
}' > /dev/null

$STIGCTL component create "$ALUCARD" Location '{
  "region": "Transylvania",
  "area": "Training Grounds",
  "x": 5.0,
  "y": 2.0,
  "altitude": 450.0
}' > /dev/null

echo "Professor Alucard: 'Marcus, you carry the blood of hunters,"
echo "                    but also the heart of a tamer.'"
echo ""
sleep 2
echo "Professor Alucard: 'Today, you will catch your first Gothic Pokemon.'"
echo ""
sleep 3

echo "--- ACT II: THE FIRST CAPTURE ---"
echo ""
sleep 1

echo "A wild Duskull appears from the cemetery mists!"
echo ""
DUSKULL=$($EXAMPLES create-gothic-pokemon duskull lesser | grep "Pokemon ID:" | awk '{print $3}')
sleep 2

echo "Examining the creature..."
echo ""
$STIGCTL component get "$DUSKULL" PokemonSpecies | jq -r '"\(.name) - \(.primary_type) type"'
$STIGCTL component get "$DUSKULL" MonsterProfile | jq -r '"Threat: \(.threat_level) | \(.description)"'
echo ""
sleep 3

echo "Marcus: 'It's both a Pokemon AND a monster... incredible!'"
echo ""
sleep 2

echo "Professor Alucard: 'Precisely. These creatures embody BOTH natures.'"
echo "Professor Alucard: 'Use your hunter skills to weaken it, then capture with a Dusk Ball.'"
echo ""
sleep 3

echo "Marcus engages in gothic battle..."
echo ""
$EXAMPLES gothic-battle "$MARCUS" "$DUSKULL" 2>/dev/null
sleep 2
echo ""

echo "The Duskull is weakened! Marcus throws a Dusk Ball..."
echo ""
sleep 2

$EXAMPLES catch-gothic-pokemon "$MARCUS" "$DUSKULL" 2>/dev/null
echo ""
sleep 2

echo "Marcus: 'I did it! My first Gothic Pokemon!'"
echo ""
sleep 2

echo "Professor Alucard: 'Well done. Now you understand: the ECS allows"
echo "                    Pokemon components and Monster components to coexist.'"
echo ""
sleep 3

echo "--- ACT III: TRAINING AND EVOLUTION ---"
echo ""
sleep 1

echo "Marcus trains with Professor Alucard..."
echo ""
$EXAMPLES train-with-mentor "$MARCUS" "$ALUCARD" 2>/dev/null
sleep 2
echo ""

echo "Professor Alucard: 'There is a relic in the old chapel that will"
echo "                    enhance your ghost Pokemon. Seek the Crimson Moon Fragment.'"
echo ""
sleep 3

echo "Marcus ventures to the Observatory Vault..."
sleep 2
echo "..."
sleep 2
echo "He finds the Crimson Moon Fragment!"
echo ""

MOON_FRAG=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$MOON_FRAG" Relic '{
  "name": "Crimson Moon Fragment",
  "category": "artifact",
  "description": "A shard of crystallized moonlight",
  "power": "Empowers Dark and Ghost moves"
}' > /dev/null

$STIGCTL component create "$MOON_FRAG" Item '{
  "name": "Crimson Moon Fragment",
  "category": "battle_item",
  "description": "Boosts Ghost and Dark type moves",
  "effect": "ghost_dark_boost_30"
}' > /dev/null

sleep 2
echo "Marcus equips the relic to his Duskull..."
echo ""
$EXAMPLES equip-relic-to-pokemon "$DUSKULL" "Crimson Moon Fragment" 2>/dev/null
sleep 2
echo ""

echo "The Duskull glows with dark power!"
echo "Its Special Attack has increased!"
echo ""
sleep 3

echo "--- ACT IV: THE TRIAL ---"
echo ""
sleep 1

echo "Professor Alucard: 'You are ready for your first trial.'"
echo "Professor Alucard: 'Face Carmilla, the Shadow Gym Leader.'"
echo ""
sleep 3

echo "Marcus travels to the Shadow Gym..."
sleep 2
echo "..."
sleep 2

CARMILLA=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$CARMILLA" NPC '{
  "name": "Carmilla the Shadow Tamer",
  "npc_type": "gym_leader",
  "dialogue": [
    "So you dare enter my domain, little hunter?",
    "My Pokemon have feasted on centuries of darkness.",
    "Impressive... Take this Shadow Badge."
  ],
  "can_battle": true,
  "defeated": false
}' > /dev/null

$STIGCTL component create "$CARMILLA" Trainer '{
  "name": "Carmilla",
  "money": 10000,
  "badges": [],
  "trainer_id": "gym_leader_shadow_001"
}' > /dev/null

$STIGCTL component create "$CARMILLA" HunterProfile '{
  "name": "Carmilla",
  "order": "Fallen Order",
  "title": "Shadow Queen",
  "renown": 150,
  "vows": ["Rule the eternal night"]
}' > /dev/null

$STIGCTL component create "$CARMILLA" Party '{
  "pokemon": []
}' > /dev/null

$STIGCTL component create "$CARMILLA" Location '{
  "region": "Transylvania",
  "area": "Shadow Gym",
  "x": 66.0,
  "y": 33.0,
  "altitude": 720.0
}' > /dev/null

echo ""
echo "Carmilla: 'So you dare enter my domain, little hunter?'"
echo ""
sleep 2
echo "Carmilla: 'My Pokemon have feasted on centuries of darkness.'"
echo ""
sleep 2
echo "Carmilla: 'Show me what a Belmont can do!'"
echo ""
sleep 3

echo "The battle begins!"
echo ""
$EXAMPLES challenge-gym "$MARCUS" "$CARMILLA" 2>/dev/null
sleep 2
echo ""

echo "Marcus stands victorious, Shadow Badge in hand."
echo ""
sleep 3

echo "--- EPILOGUE: THE POWER OF COMPOSITION ---"
echo ""
sleep 1

echo "Professor Alucard: 'Marcus, you have learned the deepest secret.'"
echo ""
sleep 2
echo "Professor Alucard: 'Pokevania exists not through new code or new classes,'"
echo "                    'but through the COMPOSITION of existing components.'"
echo ""
sleep 3
echo "Professor Alucard: 'Your entity has BOTH HunterProfile AND Trainer components.'"
echo ""
sleep 2
echo "Professor Alucard: 'Your Duskull has BOTH PokemonInstance AND MonsterState.'"
echo ""
sleep 2
echo "Professor Alucard: 'Carmilla is NPC AND Trainer AND HunterProfile at once.'"
echo ""
sleep 3
echo "Professor Alucard: 'This is the power of Entity-Component Systems:'"
echo "                    'EMERGENT behavior through COMPONENT COMPOSITION.'"
echo ""
sleep 3
echo "Professor Alucard: 'No inheritance hierarchies. No rigid class structures.'"
echo "                    'Just pure, composable data.'"
echo ""
sleep 3

echo "Marcus looks at his Dusk Ball, then at his whip."
echo "Two worlds. One system. Infinite possibilities."
echo ""
sleep 2

echo "========================================"
echo "      THE END... OR THE BEGINNING?"
echo "========================================"
echo ""
sleep 2

echo "Summary of created entities:"
echo "  Marcus Belmont (Hunter+Trainer): $MARCUS"
echo "  Professor Alucard (Mentor+NPC):  $ALUCARD"
echo "  Duskull (Pokemon+Monster):       $DUSKULL"
echo "  Carmilla (NPC+Trainer+Hunter):   $CARMILLA"
echo "  Crimson Moon (Relic+Item):       $MOON_FRAG"
echo ""
echo "All entities demonstrate component composition!"
echo "Run './examples/pokevania/pokevania-examples.sh show-hybrid-trainer $MARCUS'"
echo "to see Marcus's dual nature."
echo ""
