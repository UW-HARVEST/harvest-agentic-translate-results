#!/usr/bin/env bash
# Full verification sweep: builds both libraries, enumerates every cargo feature
# combination from Cargo.toml, and runs the differential suite plus the
# process-level stdout comparison under each one.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"
cd "$crate"

CARGO_FLAGS="${CARGO_FLAGS:---offline}"
fails=0

echo "=== 1. Build the C shared library ==="
( mkdir -p "$root/c_src/build" && cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "ok: $root/c_src/build/libdriver.so"

echo
echo "=== 2. Enumerate feature combinations ==="
# Every feature declared in [features] (excluding "default").
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                feats.append(name)
print("\n".join(feats))
PY
)
# mapfile yields a single empty element when the input is empty; drop blanks.
_f=(); for x in "${FEATURES[@]+"${FEATURES[@]}"}"; do [[ -n "$x" ]] && _f+=("$x"); done
FEATURES=("${_f[@]+"${_f[@]}"}")

# Combination list: always the default build and the no-default build; plus the
# powerset of declared features when there are any.
COMBOS=()
COMBOS+=("default|")                      # label|--features arg
COMBOS+=("no-default|__NODEFAULT__")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do (((mask >> b) & 1)) && sel+=("${FEATURES[b]}"); done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("nodefault+$joined|$joined")
  done
  COMBOS+=("all-features|__ALL__")
fi
echo "declared features: ${FEATURES[*]:-<none>}"
echo "combinations to verify: ${#COMBOS[@]}"

echo
echo "=== 3. Verify each combination ==="
for entry in "${COMBOS[@]}"; do
  label="${entry%%|*}"
  arg="${entry#*|}"
  case "$arg" in
    "")              flags=() ;;
    __NODEFAULT__)   flags=(--no-default-features) ;;
    __ALL__)         flags=(--all-features) ;;
    *)               flags=(--no-default-features --features "$arg") ;;
  esac

  echo
  echo "--- [$label] cargo build --release ${flags[*]} ---"
  if ! cargo build $CARGO_FLAGS --release "${flags[@]}" >/dev/null 2>&1; then
    echo "  BUILD FAILED"; ((fails++)); continue
  fi

  echo "--- [$label] cargo check --release --tests ---"
  cargo check $CARGO_FLAGS --release --tests "${flags[@]}" >/dev/null 2>&1 \
    || { echo "  CHECK FAILED"; ((fails++)); }

  echo "--- [$label] symbol diff (nm -D) ---"
  d=$(diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$crate/target/release/libdriver.so" | awk '{print $3}' | sort))
  if [[ -z "$d" ]]; then echo "  ok: symbol sets identical"; else
    echo "  SYMBOL DIFF NOT EMPTY:"; echo "$d"; ((fails++)); fi

  echo "--- [$label] differential suite ---"
  if cargo test $CARGO_FLAGS --release "${flags[@]}" 2>&1 | tee "${TMPDIR:-/tmp}/vt.$label.log" \
       | grep -qE '^result: [0-9]+ passed; 0 failed'; then
    echo "  ok: $(grep -E '^result:' "${TMPDIR:-/tmp}/vt.$label.log" | tail -1)"
  else
    echo "  DIFFERENTIAL SUITE FAILED"; grep -E 'FAILED|^result:' "${TMPDIR:-/tmp}/vt.$label.log" | head -20
    ((fails++))
  fi

  echo "--- [$label] process-level stdout comparison ---"
  bash "$here/compare_process_stdout.sh" 2>&1 | sed 's/^/  /' \
    || { echo "  PROCESS COMPARISON FAILED"; ((fails++)); }
done

echo
if ((fails == 0)); then
  echo "=== ALL COMBINATIONS PASSED (${#COMBOS[@]} configurations) ==="
else
  echo "=== $fails FAILURE(S) ==="
  exit 1
fi
