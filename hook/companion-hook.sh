#!/usr/bin/env bash
# Kimi Companion hook entrypoint (installed at ~/.kimi-code/companion-hook.sh).
exec python3 "$(dirname "$(readlink -f "$0")")/companion-hook.py" "$@"
