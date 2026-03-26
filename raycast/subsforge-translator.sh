#!/bin/zsh

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title SubsForge Translator
# @raycast.mode silent
# @raycast.packageName SubsForge

# Optional parameters:
# @raycast.icon 🌐
# @raycast.description Toggle le serveur de traduction NLLB-200 (port 8384)

PID_FILE="/tmp/subsforge_translator.pid"
DIR="$HOME/02_perso/whisper/subsforge/translator"

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    kill "$(cat "$PID_FILE")"
    rm -f "$PID_FILE"
    echo "Translator arrêté"
else
    cd "$DIR"
    $HOME/.local/bin/uv run uvicorn server:app --host 0.0.0.0 --port 8384 > /tmp/subsforge_translator.log 2>&1 &
    echo $! > "$PID_FILE"
    echo "Translator démarré sur :8384"
fi
