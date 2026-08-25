#!/usr/bin/env python3
"""Kimi Companion - WSL-side token bookkeeping hook.

Triggered by kimi-code [[hooks]] (Stop / Interrupt / SessionEnd). Reads the
hook payload from stdin, incrementally parses the session's wire.jsonl for
"usage.record" events and appends them to ~/.kimi-code/token-ledger.jsonl.

Also runnable standalone:
    companion-hook.py --backfill     # scan ALL existing sessions once

Design notes:
- Per-file byte offsets are kept in companion-hook.state.json, so repeat
  invocations only read new bytes.
- A flock serializes concurrent hook invocations from parallel sessions.
- After writing new records, POSTs the companion app's sync-trigger port so
  it syncs immediately (fail-open; the app's 30s polling is the fallback).
- Every failure is swallowed (exit 0): hooks are fail-open and must never
  break the CLI.
"""
import fcntl
import glob
import json
import os
import sys

KIMI_DIR = os.path.expanduser("~/.kimi-code")
LEDGER = os.path.join(KIMI_DIR, "token-ledger.jsonl")
STATE = os.path.join(KIMI_DIR, "companion-hook.state.json")
LOCK = os.path.join(KIMI_DIR, "companion-hook.lock")

# Kimi Companion 应用的联动触发端口（只绑 Windows 侧 loopback）。
# WSL2 镜像网络模式下 WSL 内可经 127.0.0.1 直达；NAT 模式下够不到，
# ping 静默失败，应用侧 30 秒轮询兜底。
TRIGGER_URL = "http://127.0.0.1:51999/sync"


def ping_companion():
    """Notify the companion app to sync the ledger right now. Fail-open."""
    import urllib.request
    try:
        urllib.request.urlopen(TRIGGER_URL, data=b"", timeout=1)
    except Exception:
        pass


def load_state():
    try:
        with open(STATE, encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return {}


def save_state(state):
    tmp = STATE + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(state, f)
    os.replace(tmp, STATE)


def process_wire(path, state, out):
    """Append new usage.record lines from one wire.jsonl to the ledger."""
    offset = state.get(path, 0)
    size = os.path.getsize(path)
    if offset > size:  # file was truncated/rotated; re-read from scratch
        offset = 0
    if offset == size:
        return 0

    # .../sessions/<wd>/<session_id>/agents/<agent>/wire.jsonl
    parts = path.split(os.sep)
    session_id = parts[-4] if len(parts) >= 5 else None
    agent_dir = parts[-2] if len(parts) >= 2 else None

    with open(path, "rb") as f:
        f.seek(offset)
        chunk = f.read()
        state[path] = f.tell()

    count = 0
    for line in chunk.splitlines():
        if b'"usage.record"' not in line:
            continue
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if event.get("type") != "usage.record":
            continue
        usage = event.get("usage") or {}
        record = {
            "ts": event.get("time"),
            "session_id": event.get("sessionId") or session_id,
            "agent": event.get("agentId") or agent_dir,
            "model": event.get("model"),
            "input": usage.get("inputOther", 0),
            "output": usage.get("output", 0),
            "cache_read": usage.get("inputCacheRead", 0),
            "cache_creation": usage.get("inputCacheCreation", 0),
        }
        out.write(json.dumps(record, ensure_ascii=False) + "\n")
        count += 1
    return count


def main():
    if "--backfill" in sys.argv:
        files = glob.glob(os.path.join(
            KIMI_DIR, "sessions", "*", "*", "agents", "*", "wire.jsonl"))
    else:
        try:
            payload = json.load(sys.stdin)
        except Exception:
            payload = {}
        session_id = payload.get("session_id")
        if not session_id:
            return 0
        files = glob.glob(os.path.join(
            KIMI_DIR, "sessions", "*", session_id, "agents", "*", "wire.jsonl"))
    if not files:
        return 0

    total = 0
    with open(LOCK, "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = load_state()
        with open(LEDGER, "a", encoding="utf-8") as out:
            for path in files:
                try:
                    total += process_wire(path, state, out)
                except Exception:
                    pass  # one bad wire file must not lose the rest
        save_state(state)
    if total > 0:
        ping_companion()
    if "--backfill" in sys.argv:
        print(f"backfill: {total} usage records -> {LEDGER}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        sys.exit(0)
