#!/bin/bash
# Mutation-testing helper: proves the differential harness actually detects a
# mistranslation, i.e. that the tests are not vacuous.
#
#   ./scripts/mutate.sh "<label>" "<old code>" "<new code>"
#
# The replacement is applied only to real code, never to the `//!` doc comment
# that quotes the original C (which contains many of the same tokens).
cd "$(dirname "$0")/.." || exit 1
ORIG=scripts/.lib.rs.orig
label="$1"; old="$2"; new="$3"
cp "$ORIG" src/lib.rs
if ! old="$old" new="$new" python3 -c '
import os, sys
old, new = os.environ["old"], os.environ["new"]
lines = open("src/lib.rs").read().split("\n")
hits = [i for i, l in enumerate(lines)
        if old in l and not l.lstrip().startswith(("//!", "//", "///"))]
if not hits:
    sys.exit("pattern not found in code: %r" % old)
i = hits[0]
lines[i] = lines[i].replace(old, new, 1)
open("src/lib.rs", "w").write("\n".join(lines))
'; then echo "$label: PATTERN-NOT-FOUND"; cp "$ORIG" src/lib.rs; exit 1; fi

# --no-fail-fast so every test binary runs: otherwise cargo stops at the first
# failing binary and the attribution below would credit whichever binary happens
# to run first rather than every test that actually detects the mutation.
out=$(timeout 900 cargo test --release --no-fail-fast 2>&1)
nfail=$(printf '%s' "$out" | grep -c '^test .* FAILED')
crash=$(printf '%s' "$out" | grep -c 'SIGSEGV\|SIGBUS\|SIGILL')
builderr=$(printf '%s' "$out" | grep -c '^error\[')
if [ "$builderr" -gt 0 ]; then
  echo "$label: DID-NOT-COMPILE (mutation invalid)"
elif [ "$nfail" -gt 0 ] || [ "$crash" -gt 0 ]; then
  # Attribute the detection to the test *binaries* that failed, so it is visible
  # whether the differential FFI tests caught it or only the in-crate unit tests.
  bins=$(printf '%s' "$out" | awk '
    /^     Running / { bin = $NF }
    /^test .* FAILED/ { print bin }' | sed 's#.*/##; s/)$//; s/-[0-9a-f]*$//' | sort -u | tr '\n' ' ')
  echo "$label: CAUGHT ($nfail failing tests, $crash crashes) via: ${bins:-<crash>}"
  printf '%s' "$out" | grep -E '^test .* FAILED' | sed 's/^/      /' | head -8
else
  echo "$label: *** NOT CAUGHT (harness gap!) ***"
fi
cp "$ORIG" src/lib.rs
