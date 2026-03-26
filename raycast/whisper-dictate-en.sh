#!/bin/zsh

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Whisper Dictée (EN)
# @raycast.mode silent
# @raycast.packageName Whisper

# Optional parameters:
# @raycast.icon 🎙️
# @raycast.description Toggle dictée vocale en anglais — copie le texte dans le presse-papier

/Users/iliad/02_perso/whisper/whisper.sh dictate --paste -l en
