#!/usr/bin/env bash
# Enforce conventional commit format with a required scope.
#
# Valid types are read from TAGS.yml and valid scopes from SCOPES.yml
# (both at the repository root).  This keeps the allowed values in one
# auditable place rather than hard-coded inside this script.
#
# Accepted format:  <type>(<scope>): <description>
#                   <type>(<scope>)!: <description>   (breaking change)
#
# Merge commits and fixup!/squash! prefixes are exempted.

set -euo pipefail

msg_file="$1"
msg="$(cat "$msg_file")"

# Allow merge commits, fixup, and squash.
if echo "$msg" | grep -qE '^(Merge |fixup! |squash! )'; then
    exit 0
fi

# Locate the repo root (script lives in scripts/, so go one level up).
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
tags_file="$repo_root/TAGS.yml"
scopes_file="$repo_root/SCOPES.yml"

# Parse top-level keys from a YAML file (lines that start with a word
# followed by a colon, excluding comment lines and indented keys).
parse_keys() {
    grep -E '^[a-zA-Z][a-zA-Z0-9_-]*:' "$1" | sed 's/:.*//' | tr '\n' '|' | sed 's/|$//'
}

if [ ! -f "$tags_file" ]; then
    echo "  ERROR: $tags_file not found — cannot validate commit type." >&2
    exit 1
fi
if [ ! -f "$scopes_file" ]; then
    echo "  ERROR: $scopes_file not found — cannot validate commit scope." >&2
    exit 1
fi

types="$(parse_keys "$tags_file")"
scopes="$(parse_keys "$scopes_file")"

pattern="^(${types})\((${scopes})\)!?: .+"

if ! echo "$msg" | grep -qE "$pattern"; then
    # Work out which part failed for a more helpful message.
    first_line="$(echo "$msg" | head -1)"
    echo ""
    echo "  ERROR: commit message does not follow the required format."
    echo ""
    echo "  Expected:  <type>(<scope>): <description>"
    echo "  Example:   feat(tangle): add allow-trailing-newline option"
    echo ""
    echo "  Valid types  (see TAGS.yml):"
    echo "    $(parse_keys "$tags_file" | tr '|' ' ')"
    echo ""
    echo "  Valid scopes (see SCOPES.yml):"
    echo "    $(parse_keys "$scopes_file" | tr '|' ' ')"
    echo ""
    echo "  Your message: $first_line"
    echo ""
    exit 1
fi
