#!/bin/zsh

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Whisper Server
# @raycast.mode silent
# @raycast.packageName Whisper

# Optional parameters:
# @raycast.icon 🌐
# @raycast.description Toggle le serveur Whisper API pour Bazarr (port 9090)

CONTAINER="whisper-asr"

if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
    docker stop "$CONTAINER" > /dev/null 2>&1
    echo "Whisper Server arrêté"
else
    docker start "$CONTAINER" > /dev/null 2>&1 || \
    docker run -d \
        --name "$CONTAINER" \
        -p 9090:9090 \
        -v "$HOME/02_perso/whisper/cache:/root/.cache/whisper" \
        -e ASR_ENGINE=faster_whisper \
        -e ASR_MODEL=large-v3-turbo \
        onerahmet/openai-whisper-asr-webservice:latest \
        --host 0.0.0.0 --port 9090 > /dev/null 2>&1
    echo "Whisper Server démarré sur :9090"
fi
