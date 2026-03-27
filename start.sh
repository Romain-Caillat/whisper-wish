#!/bin/zsh
# start.sh — Lance tout le stack SubsForge
# Usage: ./start.sh        (lance tout)
#        ./start.sh stop   (arrête tout)
#        ./start.sh status (vérifie l'état)

set -euo pipefail

SMB_ADDR="smb://guest:@192.168.1.180/media"
SMB_MOUNT="/Volumes/media"
TRANSLATOR_DIR="$HOME/02_perso/whisper/subsforge/translator"
SUBSFORGE_DIR="$HOME/02_perso/whisper/subsforge"
TRANSLATOR_PID="/tmp/subsforge_translator.pid"
SUBSFORGE_PID="/tmp/subsforge.pid"
DB="$HOME/.local/share/subsforge/subsforge.db"

is_running() {
    [[ -f "$1" ]] && kill -0 "$(cat "$1")" 2>/dev/null
}

cmd_status() {
    echo "=== SubsForge Status ==="
    # SMB
    if mount | grep -q "192.168.1.180/media"; then
        echo "✓ SMB monté sur $SMB_MOUNT"
    else
        echo "✗ SMB non monté"
    fi
    # Translator
    if is_running "$TRANSLATOR_PID"; then
        echo "✓ Translator (PID $(cat $TRANSLATOR_PID))"
    else
        echo "✗ Translator arrêté"
    fi
    # SubsForge
    if is_running "$SUBSFORGE_PID"; then
        echo "✓ SubsForge  (PID $(cat $SUBSFORGE_PID))"
    else
        echo "✗ SubsForge arrêté"
    fi
    # Stats
    if curl -s -m 2 http://localhost:8385/api/stats 2>/dev/null | grep -q total; then
        echo ""
        echo "=== Stats ==="
        curl -s http://localhost:8385/api/stats 2>/dev/null | python3 -c "
import sys,json
d=json.load(sys.stdin)
print(f\"  Total: {d['total']} | Completed: {d['completed']} | Failed: {d['failed']} | Pending: {d['pending']} | In progress: {d['in_progress']}\")
" 2>/dev/null
    fi
}

cmd_stop() {
    echo "Arrêt de SubsForge..."
    is_running "$SUBSFORGE_PID" && kill "$(cat $SUBSFORGE_PID)" 2>/dev/null && echo "  SubsForge arrêté"
    is_running "$TRANSLATOR_PID" && kill "$(cat $TRANSLATOR_PID)" 2>/dev/null && echo "  Translator arrêté"
    rm -f "$SUBSFORGE_PID" "$TRANSLATOR_PID"
    echo "Done."
}

cmd_start() {
    echo "=== Démarrage SubsForge ==="

    # 1. Monter le SMB si nécessaire
    if mount | grep -q "192.168.1.180/media"; then
        echo "✓ SMB déjà monté"
    else
        echo "→ Montage SMB..."
        open "$SMB_ADDR"
        for i in $(seq 1 15); do
            sleep 1
            if ls "$SMB_MOUNT/series" &>/dev/null; then
                echo "✓ SMB monté"
                break
            fi
            [[ $i -eq 15 ]] && { echo "✗ SMB timeout — vérifie que le serveur est allumé"; exit 1; }
        done
    fi

    # 2. Reset les jobs bloqués
    if [[ -f "$DB" ]]; then
        local stuck=$(sqlite3 "$DB" "SELECT count(*) FROM jobs WHERE status IN ('extracting','transcribing','translating')")
        if [[ "$stuck" -gt 0 ]]; then
            echo "→ Reset $stuck jobs bloqués..."
            sqlite3 "$DB" "UPDATE jobs SET status='pending', failure_reason=NULL WHERE status IN ('extracting','transcribing','translating');"
            sqlite3 "$DB" "UPDATE translations SET status='pending', failure_reason=NULL WHERE status='in_progress';"
        fi
    fi

    # 3. Lancer le translator
    if is_running "$TRANSLATOR_PID"; then
        echo "✓ Translator déjà en cours"
    else
        echo "→ Démarrage Translator (NLLB-200)..."
        cd "$TRANSLATOR_DIR"
        $HOME/.local/bin/uv run uvicorn server:app --host 0.0.0.0 --port 8384 > /tmp/subsforge_translator.log 2>&1 &
        echo $! > "$TRANSLATOR_PID"
        # Attendre que le modèle soit chargé
        for i in $(seq 1 60); do
            sleep 1
            if curl -s -m 2 http://localhost:8384/health 2>/dev/null | grep -q ok; then
                echo "✓ Translator prêt (GPU Metal)"
                break
            fi
            [[ $((i % 10)) -eq 0 ]] && echo "  chargement du modèle... ($i s)"
            [[ $i -eq 60 ]] && { echo "✗ Translator timeout"; exit 1; }
        done
    fi

    # 4. Lancer SubsForge
    if is_running "$SUBSFORGE_PID"; then
        echo "✓ SubsForge déjà en cours"
    else
        echo "→ Démarrage SubsForge..."
        cd "$SUBSFORGE_DIR"
        cargo run --release -- serve -c config.toml > /tmp/subsforge.log 2>&1 &
        echo $! > "$SUBSFORGE_PID"
        sleep 3
        if is_running "$SUBSFORGE_PID"; then
            echo "✓ SubsForge démarré (port 8385)"
        else
            echo "✗ SubsForge a crashé — voir /tmp/subsforge.log"
            exit 1
        fi
    fi

    # 5. Caffeinate
    caffeinate -s -i -d -w "$(cat $SUBSFORGE_PID)" &>/dev/null &
    echo "✓ Caffeinate actif"

    echo ""
    echo "=== Tout est lancé ==="
    echo "  API:        http://localhost:8385/api/stats"
    echo "  Translator: http://localhost:8384/health"
    echo "  Logs:       tail -f /tmp/subsforge.log"
    echo "  Arrêt:      $0 stop"
}

case "${1:-start}" in
    start)  cmd_start ;;
    stop)   cmd_stop ;;
    status) cmd_status ;;
    *)      echo "Usage: $0 [start|stop|status]" ;;
esac
