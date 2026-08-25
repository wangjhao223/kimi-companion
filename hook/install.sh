#!/usr/bin/env bash
# Install the Kimi Companion bookkeeping hook into ~/.kimi-code.
# - copies companion-hook.{sh,py} next to config.toml
# - appends [[hooks]] entries for Stop / Interrupt / SessionEnd (idempotent)
# - optionally runs a one-time backfill over all existing sessions
set -euo pipefail

KIMI_DIR="$HOME/.kimi-code"
CONFIG="$KIMI_DIR/config.toml"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

install -m 0755 "$SRC_DIR/companion-hook.sh" "$KIMI_DIR/companion-hook.sh"
install -m 0644 "$SRC_DIR/companion-hook.py" "$KIMI_DIR/companion-hook.py"
echo "installed: $KIMI_DIR/companion-hook.{sh,py}"

if grep -q "companion-hook" "$CONFIG" 2>/dev/null; then
    echo "config.toml already contains companion-hook entries, skipped"
else
    cp "$CONFIG" "$CONFIG.$(date +%Y%m%d-%H%M%S).bak"
    cat >> "$CONFIG" <<'EOF'

# Kimi Companion: token bookkeeping (https://github.com/kimi-companion)
[[hooks]]
event = "Stop"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10

[[hooks]]
event = "Interrupt"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10

[[hooks]]
event = "SessionEnd"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10
EOF
    echo "config.toml updated (backup created)"
fi

if [[ "${1:-}" == "--backfill" ]]; then
    python3 "$KIMI_DIR/companion-hook.py" --backfill
fi
