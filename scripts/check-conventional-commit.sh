#!/usr/bin/env bash
# Enforce conventional commit format with a required scope:
#   <type>(<scope>): <description>
#
# Allowed types: feat fix docs style refactor perf test build ci chore revert
# Breaking changes are permitted via an optional `!` before the colon.
# Merge commits and fixup!/squash! commits are exempted.

set -euo pipefail

msg_file="$1"
msg="$(cat "$msg_file")"

# Allow merge commits, fixup, and squash.
if echo "$msg" | grep -qE '^(Merge |fixup! |squash! )'; then
    exit 0
fi

pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)\([^)]+\)!?: .+'

if ! echo "$msg" | grep -qE "$pattern"; then
    echo ""
    echo "  ERROR: commit message does not follow Conventional Commits with a required scope."
    echo ""
    echo "  Expected format:  <type>(<scope>): <description>"
    echo "  Example:          feat(tangle): add allow-trailing-newline option"
    echo ""
    echo "  Allowed types: feat fix docs style refactor perf test build ci chore revert"
    echo "  The scope is REQUIRED and must be non-empty."
    echo ""
    echo "  Your message:"
    echo "    $msg" | head -1
    echo ""
    exit 1
fi
