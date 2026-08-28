#!/usr/bin/env bash
# Phase D: run `cargo check` + the whole differential suite under EVERY feature
# combination the crate declares, in both the debug and the release profile.
#
# `Cargo.toml` declares no `[features]` and no optional dependencies, so the
# combination set is {<default>, --no-default-features}.  The script derives it
# from Cargo.toml rather than hard-coding it, so it keeps working if features
# are added later.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$root" || exit 1

# ---- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'(?ms)^\[features\]\s*(.*?)(?=^\[|\Z)', txt)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                feats.append(name)
print('\n'.join(feats))
PY
)

# drop the empty element mapfile produces when there are no features
tmp=(); for f in "${FEATURES[@]+"${FEATURES[@]}"}"; do [ -n "$f" ] && tmp+=("$f"); done
FEATURES=("${tmp[@]+"${tmp[@]}"}")

# power set of FEATURES (plus the plain default build)
COMBOS=("<default>" "<none>")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$sel")
  done
fi

echo "declared features : ${FEATURES[*]:-(none)}"
echo "combinations      : ${#COMBOS[@]}"
echo

status=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") args=() ;;
    "<none>")    args=(--no-default-features) ;;
    *)           args=(--no-default-features --features "$combo") ;;
  esac

  for profile in "" --release; do
    label="combo=$combo profile=${profile:-debug}"
    echo "=== $label"

    if ! cargo check --offline "${args[@]}" $profile >/dev/null 2>&1; then
      echo "    cargo check FAILED"; status=1; continue
    fi
    if ! cargo build --offline "${args[@]}" $profile >/dev/null 2>&1; then
      echo "    cargo build FAILED"; status=1; continue
    fi

    # The suite always loads the .so via libloading; point it at the .so that
    # belongs to the profile just built, and run the (fast) debug harness.
    prof_dir="target/$([ -n "$profile" ] && echo release || echo debug)"
    so="$root/$prof_dir/libdequantize_granule_lib.so"
    if [ ! -f "$so" ]; then echo "    missing $so"; status=1; continue; fi

    out=$(RUST_SO="$so" cargo test --offline "${args[@]}" -- --test-threads=8 2>&1)
    if [ $? -ne 0 ]; then
      echo "    TESTS FAILED against $so"
      echo "$out" | tail -25 | sed 's/^/      /'
      status=1
    else
      echo "    $(echo "$out" | grep -c '^test result: ok') test binaries ok:"
      echo "$out" | grep '^test result' | sed 's/^/      /'
    fi
  done
done

echo
if [ $status -eq 0 ]; then
  echo "ALL feature combinations x profiles PASS"
else
  echo "FAILURES detected"
fi
exit $status
