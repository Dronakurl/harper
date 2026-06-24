#!/usr/bin/env python3

# Script to add strategic compound-forming words to German dictionary
# These words will enable the existing compound word system to work better

compound_components = [
    # Common first components
    "Arbeit",      # work
    "Auto",         # car
    "Bild",        # picture/image
    "Bund",        # federation/league
    "Bundes",      # federal (genitive form)
    "Fest",        # festival/fixed
    "Fest",        # festival/fixed
    "Haus",        # house
    "Kinder",      # children
    "Lebens",      # life (genitive)
    "Schule",      # school
    "Stadt",       # city
    "Straße",      # street
    "Wasser",      # water
    "Welt",        # world
    "Zeit",        # time
    
    # Common second components
    "Anlage",      # facility/installation
    "Ansicht",     # view
    "Arbeit",      # work
    "Bahn",        # railway/track
    "Bild",        # picture
    "Buch",        # book
    "Fahrzeug",    # vehicle
    "Geschichte",  # history/story
    "Haus",        # house
    "Karte",       # card/map
    "Kraft",       # power/force
    "Lehre",       # teaching/doctrine
    "Mittel",      # means/agent
    "Platz",       # place/square
    "Punkt",       # point
    "Raum",        # space/room
    "Recht",       # law/right
    "Schule",      # school
    "Stelle",      # place/position
    "Straße",      # street
    "System",      # system
    "Teil",        # part
    "Versicherung", # insurance
    "Werk",        # work/factory
    "Zeit",        # time
    "Zimmer",      # room
]

# Generate dictionary entries with annotations
print("# Strategic compound-forming words to add to german_proper_final.dict")
print("# These will enable the existing compound word system to recognize more compounds")
print()

# Count existing words in the dictionary
with open('/home/konrad/gallery/harper/harper-core/src/language/german/german_proper_final.dict', 'r') as f:
    lines = f.readlines()
    current_count = int(lines[0].strip())
    print(f"Current word count: {current_count}")

# Add new words
new_words = []
for word in compound_components:
    # Determine gender and annotation based on common patterns
    if word.endswith("ung"):  # -ung nouns are typically feminine
        annotation = "~NF"
    elif word.endswith("e"):  # -e endings often feminine
        annotation = "~NF"
    elif word.endswith("heit") or word.endswith("keit"):  # abstract nouns feminine
        annotation = "~NF"
    elif word in ["Arbeit", "Zeit", "Schule", "Straße", "Karte", "Stelle", "Ansicht", "Anlage"]:
        annotation = "~NF"
    elif word in ["Bild", "Bund", "Haus", "Werk", "Teil", "Punkt", "Raum", "System"]:
        annotation = "~NZ"  # neuter
    elif word in ["Bahn", "Platz", "Recht", "Versicherung", "Mittel", "Kraft"]:
        annotation = "~NF"
    elif word in ["Fahrzeug", "Wasser", "Geschichte"]:
        annotation = "~NZ"
    else:
        annotation = "~NM"  # default to masculine
    
    new_words.append(f"{word}/{annotation} # compound component")

print(f"Adding {len(new_words)} new compound-forming words")
print(f"New word count will be: {current_count + len(new_words)}")
print()

# Print the new entries
for entry in sorted(set(new_words)):  # Remove duplicates and sort
    print(entry)