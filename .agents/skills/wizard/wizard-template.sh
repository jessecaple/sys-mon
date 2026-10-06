#!/usr/bin/env bash
set -euo pipefail

TOTAL_STAGES=1
CURRENT_STAGE=0

say(){ printf '%s\n' "$*"; }
die(){ printf 'Error: %s\n' "$*" >&2; exit 1; }
banner(){ printf '\n=== %s ===\n' "$*"; }
stage(){ CURRENT_STAGE=$((CURRENT_STAGE+1)); printf '\n[%s/%s] %s\n' "$CURRENT_STAGE" "$TOTAL_STAGES" "$*"; }
step(){ printf '  • %s\n' "$*"; }
confirm(){ local reply; read -r -p "$1 [y/N] " reply; [[ "$reply" =~ ^[Yy]$ ]]; }
ask(){ local __name=$1 prompt=$2 value; read -r -p "$prompt " value; [[ -n "$value" ]] || die 'A value is required'; printf -v "$__name" '%s' "$value"; }
ask_secret(){ local __name=$1 prompt=$2 value; read -r -s -p "$prompt " value; printf '\n'; [[ -n "$value" ]] || die 'A secret is required'; printf -v "$__name" '%s' "$value"; }
open_url(){ local url=$1; say "Open: $url"; if command -v open >/dev/null 2>&1; then open "$url" >/dev/null 2>&1 || true; elif command -v xdg-open >/dev/null 2>&1; then xdg-open "$url" >/dev/null 2>&1 || true; fi; }
write_env(){ local key=$1 value=$2 file=${3:-.env}; if [[ -e "$file" ]] && grep -qE "^${key}=" "$file" && ! confirm "$key already exists in $file. Replace it?"; then return; fi; local tmp; tmp=$(mktemp); if [[ -e "$file" ]]; then grep -vE "^${key}=" "$file" >"$tmp" || true; fi; printf '%s=%s\n' "$key" "$value" >>"$tmp"; chmod 600 "$tmp"; mv "$tmp" "$file"; }
finish(){ [[ "$CURRENT_STAGE" -eq "$TOTAL_STAGES" ]] || die "Expected $TOTAL_STAGES stages, ran $CURRENT_STAGE"; banner 'Complete'; }

banner 'Setup wizard'
# Replace this example with confirmed, provider-specific stages.
stage 'Example human boundary'
step 'Follow the verified primary documentation shown by this wizard.'
# open_url 'https://provider.example/docs'
# ask VALUE 'Enter the non-secret value:'
# ask_secret SECRET 'Enter the secret value:'
# confirm 'Perform the irreversible action now?' || die 'Cancelled safely'
finish
