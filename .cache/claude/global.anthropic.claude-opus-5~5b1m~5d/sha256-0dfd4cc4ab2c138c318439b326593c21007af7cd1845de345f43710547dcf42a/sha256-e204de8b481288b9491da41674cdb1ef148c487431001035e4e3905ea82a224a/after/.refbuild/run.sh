#!/bin/bash
# usage: run.sh <binary> <stdin-file> [args...]
BIN="$1"; shift
IN="$1"; shift
"$BIN" "$@" < "$IN" > .out 2> .err
echo "rc=$?"
echo "--- stdout ---"; cat .out
echo "--- stderr ---"; cat .err
