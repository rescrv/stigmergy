#!/usr/bin/env bash
# An original adventure story using the Pokemon-like game system
# Run setup-pokemon-world.sh first

set -euo pipefail

# Build stigctl once at the start to avoid cargo messages later
echo "Building stigctl..."
cargo build --bin stigctl 2>&1 | grep -v "Compiling\|Finished" || true
echo ""

STIGCTL="${STIGCTL:-./target/debug/stigctl}"
BASE_URL="${BASE_URL:-http://localhost:8080}"

if [ -n "$BASE_URL" ]; then
    STIGCTL="$STIGCTL --base-url=$BASE_URL"
fi

# Helper function to pause for dramatic effect
pause() {
    sleep "${1:-2}"
}

# Helper function to show divider
divider() {
    echo ""
    echo "================================"
    echo ""
}

clear

cat <<'EOF'
╔══════════════════════════════════════════════════════╗
║                                                      ║
║          THE TALE OF THE WANDERING TRAINER          ║
║                                                      ║
║              A Stigmergy Adventure                   ║
║                                                      ║
╚══════════════════════════════════════════════════════╝

EOF

pause 2

echo "In a world where creatures and humans live in harmony..."
pause 2
echo "There exists a young explorer named Morgan."
pause 2
echo "Today begins their journey."
pause 2

divider

echo ">> Creating Morgan's character..."
MORGAN=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')
pause 1

$STIGCTL component create "$MORGAN" Trainer '{
  "name": "Morgan",
  "money": 500,
  "badges": [],
  "trainer_id": "trainer_morgan_001"
}' > /dev/null

$STIGCTL component create "$MORGAN" Position '{
  "x": 0.0,
  "y": 0.0,
  "map": "whispering_grove",
  "facing": "north"
}' > /dev/null

$STIGCTL component create "$MORGAN" Party '{
  "pokemon": []
}' > /dev/null

$STIGCTL component create "$MORGAN" Inventory '{
  "items": {
    "potion": 3,
    "pokeball": 5
  }
}' > /dev/null

echo "✓ Morgan is ready for adventure!"
pause 2

divider

echo "Morgan stepped into the tall grass of Whispering Grove."
pause 2
echo "The morning sun filtered through the ancient trees."
pause 2
echo ""
echo "Suddenly, rustling in the grass!"
pause 2
echo ""
echo "A wild creature appears..."
pause 2

echo ">> A small purple creature with bright eyes emerges!"
pause 1

WILD_RATTATA=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$WILD_RATTATA" PokemonSpecies '{
  "name": "Rattata",
  "pokedex_number": 19,
  "primary_type": "Normal",
  "secondary_type": null,
  "base_hp": 30,
  "base_attack": 56,
  "base_defense": 35,
  "base_sp_attack": 25,
  "base_sp_defense": 35,
  "base_speed": 72,
  "evolution_level": 20,
  "evolves_into": "Raticate"
}' > /dev/null

$STIGCTL component create "$WILD_RATTATA" PokemonInstance '{
  "nickname": null,
  "level": 3,
  "current_hp": 14,
  "max_hp": 14,
  "attack": 9,
  "defense": 7,
  "sp_attack": 6,
  "sp_defense": 7,
  "speed": 12,
  "experience": 0,
  "experience_to_next_level": 75,
  "status": null,
  "friendship": 50
}' > /dev/null

$STIGCTL component create "$WILD_RATTATA" MoveSet '{
  "moves": ["Tackle", "Tail Whip"]
}' > /dev/null

echo "✓ Wild Rattata appeared! (Level 3)"
pause 2

divider

echo "Morgan: 'I have no companions yet. I must capture this creature carefully.'"
pause 2
echo ""
echo "Morgan reaches for a capture sphere..."
pause 2
echo ""
echo ">> Using Pokeball..."
pause 1

# Simulate capture by adding to party
CURRENT_PARTY=$($STIGCTL component get "$MORGAN" Party 2>/dev/null)
UPDATED_PARTY=$(echo "$CURRENT_PARTY" | jq --arg pid "$WILD_RATTATA" '.pokemon += [$pid]')
$STIGCTL component update "$MORGAN" Party "$UPDATED_PARTY" > /dev/null 2>&1

# Update inventory
CURRENT_INV=$($STIGCTL component get "$MORGAN" Inventory 2>/dev/null)
UPDATED_INV=$(echo "$CURRENT_INV" | jq '.items.pokeball -= 1')
$STIGCTL component update "$MORGAN" Inventory "$UPDATED_INV" > /dev/null 2>&1

echo "..."
pause 1
echo "..."
pause 1
echo "✓ Success! Rattata was captured!"
pause 2

echo ""
echo "Morgan: 'Welcome, little friend. I shall call you Whisker.'"
pause 2

# Give nickname
RATTATA_DATA=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
NICKNAMED_DATA=$(echo "$RATTATA_DATA" | jq '.nickname = "Whisker"')
$STIGCTL component update "$WILD_RATTATA" PokemonInstance "$NICKNAMED_DATA" > /dev/null 2>&1

divider

echo "Morgan continued through the grove with Whisker at their side."
pause 2
echo "The path wound deeper into the forest."
pause 2
echo ""
echo "Suddenly, a voice called out!"
pause 2
echo ""
echo "'Hey! You there! Wanna battle?'"
pause 2

divider

echo ">> A rival trainer approaches!"
pause 1

RIVAL=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$RIVAL" Trainer '{
  "name": "Scout Riley",
  "money": 300,
  "badges": [],
  "trainer_id": "trainer_riley_047"
}' > /dev/null

$STIGCTL component create "$RIVAL" Position '{
  "x": 5.0,
  "y": 5.0,
  "map": "whispering_grove",
  "facing": "south"
}' > /dev/null

echo "✓ Scout Riley wants to battle!"
pause 2

echo ""
echo "Riley: 'I just started my journey yesterday. Let me show you what I've learned!'"
pause 2

echo ">> Riley sends out their companion..."
pause 1

RIVAL_PIDGEY=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$RIVAL_PIDGEY" PokemonSpecies '{
  "name": "Pidgey",
  "pokedex_number": 16,
  "primary_type": "Normal",
  "secondary_type": "Flying",
  "base_hp": 40,
  "base_attack": 45,
  "base_defense": 40,
  "base_sp_attack": 35,
  "base_sp_defense": 35,
  "base_speed": 56,
  "evolution_level": 18,
  "evolves_into": "Pidgeotto"
}' > /dev/null

$STIGCTL component create "$RIVAL_PIDGEY" PokemonInstance '{
  "nickname": "Swift",
  "level": 4,
  "current_hp": 19,
  "max_hp": 19,
  "attack": 10,
  "defense": 9,
  "sp_attack": 8,
  "sp_defense": 8,
  "speed": 12,
  "experience": 0,
  "experience_to_next_level": 100,
  "status": null,
  "friendship": 70
}' > /dev/null

$STIGCTL component create "$RIVAL_PIDGEY" MoveSet '{
  "moves": ["Tackle", "Sand Attack", "Gust"]
}' > /dev/null

echo "✓ Riley sent out Swift the Pidgey! (Level 4)"
pause 2

divider

echo "BATTLE BEGIN!"
pause 1
echo ""
echo "Morgan: 'Go, Whisker!'"
pause 2

echo ""
echo "--- ROUND 1 ---"
pause 1
echo ""
echo "Whisker used Tackle!"
pause 1

# Apply damage to Pidgey
PIDGEY_STATS=$($STIGCTL component get "$RIVAL_PIDGEY" PokemonInstance 2>/dev/null)
DAMAGED_PIDGEY=$(echo "$PIDGEY_STATS" | jq '.current_hp = (.current_hp - 7)')
$STIGCTL component update "$RIVAL_PIDGEY" PokemonInstance "$DAMAGED_PIDGEY" > /dev/null 2>&1

echo ">> Pidgey took 7 damage! (12/19 HP remaining)"
pause 2

echo ""
echo "Swift used Gust!"
pause 1

# Apply damage to Whisker
WHISKER_STATS=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
DAMAGED_WHISKER=$(echo "$WHISKER_STATS" | jq '.current_hp = (.current_hp - 5)')
$STIGCTL component update "$WILD_RATTATA" PokemonInstance "$DAMAGED_WHISKER" > /dev/null 2>&1

echo ">> Whisker took 5 damage! (9/14 HP remaining)"
pause 2

echo ""
echo "--- ROUND 2 ---"
pause 1
echo ""
echo "Whisker used Tackle again!"
pause 1

PIDGEY_STATS=$($STIGCTL component get "$RIVAL_PIDGEY" PokemonInstance 2>/dev/null)
DAMAGED_PIDGEY=$(echo "$PIDGEY_STATS" | jq '.current_hp = (.current_hp - 8)')
$STIGCTL component update "$RIVAL_PIDGEY" PokemonInstance "$DAMAGED_PIDGEY" > /dev/null 2>&1

echo ">> A critical hit! Pidgey took 8 damage! (4/19 HP remaining)"
pause 2

echo ""
echo "Swift tried to use Gust, but it missed!"
pause 2

echo ""
echo "--- ROUND 3 ---"
pause 1
echo ""
echo "Whisker used Tail Whip!"
pause 1
echo ">> Swift's defense fell!"
pause 2

echo ""
echo "Swift used Tackle!"
pause 1

WHISKER_STATS=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
DAMAGED_WHISKER=$(echo "$WHISKER_STATS" | jq '.current_hp = (.current_hp - 4)')
$STIGCTL component update "$WILD_RATTATA" PokemonInstance "$DAMAGED_WHISKER" > /dev/null 2>&1

echo ">> Whisker took 4 damage! (5/14 HP remaining)"
pause 2

echo ""
echo "--- FINAL ROUND ---"
pause 1
echo ""
echo "Morgan: 'Finish this, Whisker!'"
pause 1
echo ""
echo "Whisker used Tackle with all its might!"
pause 2

PIDGEY_STATS=$($STIGCTL component get "$RIVAL_PIDGEY" PokemonInstance 2>/dev/null)
FAINTED_PIDGEY=$(echo "$PIDGEY_STATS" | jq '.current_hp = 0')
$STIGCTL component update "$RIVAL_PIDGEY" PokemonInstance "$FAINTED_PIDGEY" > /dev/null 2>&1

echo ">> Swift fainted!"
pause 2

divider

echo "BATTLE WON!"
pause 1
echo ""
echo "Riley: 'Wow, you're really good! I need to train more.'"
pause 2
echo ""
echo "Riley: 'Here, take this as a reward for beating me.'"
pause 2

# Update Morgan's money
MORGAN_TRAINER=$($STIGCTL component get "$MORGAN" Trainer 2>/dev/null)
UPDATED_TRAINER=$(echo "$MORGAN_TRAINER" | jq '.money += 200')
$STIGCTL component update "$MORGAN" Trainer "$UPDATED_TRAINER" > /dev/null 2>&1

echo ">> Received 200 coins!"
pause 2

echo ""
echo "Whisker is glowing!"
pause 2

# Level up Whisker
WHISKER_STATS=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
LEVELED_WHISKER=$(echo "$WHISKER_STATS" | jq '
    .level = 4 |
    .max_hp = 16 |
    .current_hp = 7 |
    .attack += 2 |
    .defense += 1 |
    .sp_attack += 1 |
    .sp_defense += 1 |
    .speed += 2 |
    .experience = 0 |
    .experience_to_next_level = 100
')
$STIGCTL component update "$WILD_RATTATA" PokemonInstance "$LEVELED_WHISKER" > /dev/null 2>&1

echo "✓ Whisker grew to Level 4!"
pause 2

divider

echo "Morgan: 'Good work, Whisker. Let's rest for a moment.'"
pause 2
echo ""
echo ">> Using Potion on Whisker..."
pause 1

# Heal with potion
WHISKER_STATS=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
HEALED_WHISKER=$(echo "$WHISKER_STATS" | jq '.current_hp = ([.current_hp + 20, .max_hp] | min)')
$STIGCTL component update "$WILD_RATTATA" PokemonInstance "$HEALED_WHISKER" > /dev/null 2>&1

CURRENT_INV=$($STIGCTL component get "$MORGAN" Inventory 2>/dev/null)
UPDATED_INV=$(echo "$CURRENT_INV" | jq '.items.potion -= 1')
$STIGCTL component update "$MORGAN" Inventory "$UPDATED_INV" > /dev/null 2>&1

echo "✓ Whisker restored to full health!"
pause 2

divider

echo "Riley: 'Hey, there's something I heard about in these woods...'"
pause 2
echo "Riley: 'An old guardian lives near the ancient oak tree.'"
pause 2
echo "Riley: 'They say it watches over travelers and grants wisdom to those who seek it.'"
pause 2
echo ""
echo "Morgan: 'A guardian? That sounds like someone worth meeting.'"
pause 2

divider

echo "Morgan and Whisker ventured deeper into the forest."
pause 2
echo "The trees grew taller and the air shimmered with energy."
pause 2
echo ""
echo "At the heart of the grove stood a massive oak, its trunk wide as a house."
pause 2
echo ""
echo "And there, sitting peacefully beneath it..."
pause 2

echo ">> A mysterious figure appears..."
pause 1

GUARDIAN=$($STIGCTL entity create | grep "Created entity:" | awk '{print $3}')

$STIGCTL component create "$GUARDIAN" NPC '{
  "name": "Elder Sage",
  "npc_type": "generic",
  "dialogue": [
    "Welcome, young traveler.",
    "I sense great potential in you and your companion.",
    "The path ahead will be challenging, but bonds of friendship will light your way.",
    "Remember: true strength comes not from power alone, but from understanding and respect.",
    "Go forth, and may your journey be filled with wonder."
  ],
  "can_battle": false,
  "defeated": false
}' > /dev/null

$STIGCTL component create "$GUARDIAN" Position '{
  "x": 10.0,
  "y": 10.0,
  "map": "whispering_grove",
  "facing": "south"
}' > /dev/null

echo "✓ Elder Sage greets you"
pause 2

echo ""
echo "Elder Sage: 'Welcome, young traveler.'"
pause 2
echo "Elder Sage: 'I sense great potential in you and your companion.'"
pause 2
echo ""
echo "The Elder looked at Whisker with kind eyes."
pause 2
echo ""
echo "Elder Sage: 'Your Rattata has a brave heart. Cherish this bond.'"
pause 2
echo "Elder Sage: 'The path ahead will be challenging, but bonds of friendship will light your way.'"
pause 2
echo "Elder Sage: 'Remember: true strength comes not from power alone, but from understanding and respect.'"
pause 2
echo ""
echo "Morgan nodded thoughtfully, feeling the weight and warmth of those words."
pause 2
echo ""
echo "Elder Sage: 'Go forth, and may your journey be filled with wonder.'"
pause 2

divider

echo "As the sun began to set, Morgan and Whisker left the grove."
pause 2
echo "The path ahead stretched toward distant mountains."
pause 2
echo ""
echo "Morgan: 'This is just the beginning, isn't it, Whisker?'"
pause 2
echo ""
echo "Whisker chirped in agreement, tail swishing with excitement."
pause 2
echo ""
echo "Together, they walked toward the horizon..."
pause 2
echo "...where countless adventures awaited."
pause 3

divider

cat <<EOF
╔══════════════════════════════════════════════════════╗
║                                                      ║
║                  CHAPTER ONE COMPLETE                ║
║                                                      ║
║              Morgan's Stats:                         ║
EOF

echo -n "║              Money: "
FINAL_TRAINER=$($STIGCTL component get "$MORGAN" Trainer 2>/dev/null)
FINAL_MONEY=$(echo "$FINAL_TRAINER" | jq -r '.money')
printf "%-35s ║\n" "$FINAL_MONEY coins"

echo -n "║              Potions: "
FINAL_INV=$($STIGCTL component get "$MORGAN" Inventory 2>/dev/null)
FINAL_POTIONS=$(echo "$FINAL_INV" | jq -r '.items.potion')
printf "%-33s ║\n" "$FINAL_POTIONS"

echo -n "║              Pokeballs: "
FINAL_BALLS=$(echo "$FINAL_INV" | jq -r '.items.pokeball')
printf "%-31s ║\n" "$FINAL_BALLS"

cat <<EOF
║                                                      ║
║              Whisker's Stats:                        ║
EOF

FINAL_WHISKER=$($STIGCTL component get "$WILD_RATTATA" PokemonInstance 2>/dev/null)
echo -n "║              Level: "
LEVEL=$(echo "$FINAL_WHISKER" | jq -r '.level')
printf "%-35s ║\n" "$LEVEL"

echo -n "║              HP: "
HP=$(echo "$FINAL_WHISKER" | jq -r '"\(.current_hp)/\(.max_hp)"')
printf "%-38s ║\n" "$HP"

cat <<'EOF'
║                                                      ║
║              Entity IDs for reference:               ║
EOF

echo "║              Morgan: $(printf '%-31s' "$MORGAN") ║"
echo "║              Whisker: $(printf '%-30s' "$WILD_RATTATA") ║"

cat <<'EOF'
║                                                      ║
║         To be continued in future adventures...      ║
║                                                      ║
╚══════════════════════════════════════════════════════╝

EOF

echo "Thank you for experiencing this story!"
echo ""
echo "All entities remain in the database for you to continue the adventure."
echo "Use the pokemon-examples.sh script to interact with them further!"
echo ""
