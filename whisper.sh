#!/bin/zsh
# whisper.sh — Wrapper pour whisper-cpp
# Usage:
#   whisper.sh transcribe <fichier_audio> [-l langue]
#   whisper.sh video <fichier_video> [-l langue] [--srt]
#   whisper.sh dictate [-l langue] [--paste]

set -euo pipefail

MODEL="$HOME/02_perso/whisper/models/ggml-large-v3-turbo.bin"
WHISPER_CLI="/opt/homebrew/bin/whisper-cli"
WHISPER_STREAM="/opt/homebrew/bin/whisper-stream"
FFMPEG="/opt/homebrew/bin/ffmpeg"
PID_FILE="/tmp/whisper_dictate.pid"
OUTPUT_FILE="/tmp/whisper_dictation.txt"
DEFAULT_LANG="fr"

notify() {
    osascript -e "display notification \"$1\" with title \"Whisper\""
}

cmd_transcribe() {
    local file="" lang="$DEFAULT_LANG" extra_args=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -l|--language) lang="$2"; shift 2 ;;
            --srt) extra_args+=(--output-srt); shift ;;
            --txt) extra_args+=(--output-txt); shift ;;
            --timestamps) extra_args+=(--timestamps); shift ;;
            *) file="$1"; shift ;;
        esac
    done

    if [[ -z "$file" ]]; then
        echo "Usage: whisper.sh transcribe <fichier> [-l langue]" >&2
        exit 1
    fi

    "$WHISPER_CLI" \
        -m "$MODEL" \
        -l "$lang" \
        -f "$file" \
        --no-prints \
        "${extra_args[@]}" 2>/dev/null
}

cmd_video() {
    local file="" lang="$DEFAULT_LANG" srt=false extra_args=()
    local tmp_audio="/tmp/whisper_audio_$$.wav"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -l|--language) lang="$2"; shift 2 ;;
            --srt) srt=true; extra_args+=(--output-srt); shift ;;
            *) file="$1"; shift ;;
        esac
    done

    if [[ -z "$file" ]]; then
        echo "Usage: whisper.sh video <fichier_video> [-l langue] [--srt]" >&2
        exit 1
    fi

    # Extraire l'audio en WAV 16kHz mono
    "$FFMPEG" -i "$file" -vn -ar 16000 -ac 1 -c:a pcm_s16le "$tmp_audio" -y -loglevel error

    "$WHISPER_CLI" \
        -m "$MODEL" \
        -l "$lang" \
        -f "$tmp_audio" \
        --no-prints \
        "${extra_args[@]}" 2>/dev/null

    rm -f "$tmp_audio"
}

cmd_dictate() {
    local lang="$DEFAULT_LANG" do_paste=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -l|--language) lang="$2"; shift 2 ;;
            --paste) do_paste=true; shift ;;
            *) shift ;;
        esac
    done

    # Toggle : si déjà en cours, on arrête
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid" 2>/dev/null
            sleep 0.5

            if [[ -f "$OUTPUT_FILE" ]]; then
                # Nettoyer la sortie : enlever timestamps et artefacts
                local text
                text=$(sed 's/\[.*\]//g; /^$/d; s/^[[:space:]]*//; s/[[:space:]]*$//' "$OUTPUT_FILE" | tr '\n' ' ' | sed 's/  */ /g; s/^ *//; s/ *$//')

                if [[ -n "$text" ]]; then
                    printf '%s' "$text" | pbcopy
                    notify "Copié dans le presse-papier"

                    if $do_paste; then
                        sleep 0.2
                        osascript -e 'tell application "System Events" to keystroke "v" using command down'
                    fi
                else
                    notify "Aucun texte détecté"
                fi
            fi

            rm -f "$PID_FILE" "$OUTPUT_FILE"
            echo "stopped"
            return 0
        else
            rm -f "$PID_FILE"
        fi
    fi

    # Démarrer la dictée
    rm -f "$OUTPUT_FILE"

    "$WHISPER_STREAM" \
        -m "$MODEL" \
        -l "$lang" \
        --file "$OUTPUT_FILE" \
        --step 3000 \
        --length 10000 \
        &>/dev/null &

    echo $! > "$PID_FILE"
    notify "Dictée démarrée ($lang)"
    echo "started"
}

# Point d'entrée
case "${1:-help}" in
    transcribe) shift; cmd_transcribe "$@" ;;
    video)      shift; cmd_video "$@" ;;
    dictate)    shift; cmd_dictate "$@" ;;
    help|--help|-h)
        echo "Usage:"
        echo "  whisper.sh transcribe <fichier> [-l langue]"
        echo "  whisper.sh video <fichier_video> [-l langue] [--srt]"
        echo "  whisper.sh dictate [-l langue] [--paste]"
        echo ""
        echo "Langues: fr, en, es, de, it, pt, zh, ja, ko, ar, ..."
        ;;
    *) echo "Commande inconnue: $1" >&2; exit 1 ;;
esac
