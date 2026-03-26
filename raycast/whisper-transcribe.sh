#!/bin/zsh

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Whisper Transcrire Fichier
# @raycast.mode fullOutput
# @raycast.packageName Whisper

# Optional parameters:
# @raycast.icon 📝
# @raycast.description Transcrire un fichier audio ou vidéo
# @raycast.argument1 { "type": "text", "placeholder": "Chemin du fichier" }

file="$1"

if [[ "$file" == *.mp4 || "$file" == *.mkv || "$file" == *.avi || "$file" == *.mov || "$file" == *.webm ]]; then
    /Users/iliad/02_perso/whisper/whisper.sh video "$file"
else
    /Users/iliad/02_perso/whisper/whisper.sh transcribe "$file"
fi
