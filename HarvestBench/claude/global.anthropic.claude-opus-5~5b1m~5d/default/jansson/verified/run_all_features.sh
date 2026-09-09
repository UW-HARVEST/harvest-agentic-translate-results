#!/bin/sh
# Run the whole differential suite under EVERY cargo feature combination.
#
# `Cargo.toml` declares no [features] section and `src/` contains no
# `cfg(feature = ...)`, so the feature powerset is the single empty set. The
# three invocations below are therefore all equivalent — this script exists to
# PROVE that mechanically rather than by assertion, and it will start covering
# real combinations automatically if features are ever added.
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT/translation"

echo "== declared features =="
FEATURES=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ' || true)
if [ -z "$FEATURES" ]; then
  echo "(none declared)"
else
  echo "$FEATURES"
fi
echo "== cfg(feature) uses in src/ =="
grep -rn 'cfg(feature' src/ || echo "(none)"

# Build the C .so once.
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

# Enumerate the powerset of declared features (empty set included).
COMBOS_FILE=$(mktemp)
printf '%s\n' "__default__" > "$COMBOS_FILE"
printf '%s\n' "__nodefault__" >> "$COMBOS_FILE"
printf '%s\n' "__all__" >> "$COMBOS_FILE"
if [ -n "$FEATURES" ]; then
  # powerset via binary counting
  set -- $FEATURES
  N=$#
  MAX=$(( (1 << N) - 1 ))
  i=0
  while [ $i -le $MAX ]; do
    combo=""
    j=0
    for f in $FEATURES; do
      if [ $(( (i >> j) & 1 )) -eq 1 ]; then
        combo="$combo,$f"
      fi
      j=$((j+1))
    done
    combo=$(echo "$combo" | sed 's/^,//')
    printf '%s\n' "${combo:-__empty__}" >> "$COMBOS_FILE"
    i=$((i+1))
  done
fi

STATUS=0
while read -r combo; do
  case "$combo" in
    __default__)   ARGS="" ;;
    __nodefault__) ARGS="--no-default-features" ;;
    __all__)       ARGS="--all-features" ;;
    __empty__)     ARGS="--no-default-features" ;;
    *)             ARGS="--no-default-features --features $combo" ;;
  esac
  echo
  echo "======================================================================"
  echo "== FEATURE COMBO: $combo   (cargo $ARGS)"
  echo "======================================================================"
  # Rebuild the Rust .so for this combo, then run every test against it.
  # shellcheck disable=SC2086
  if cargo build --offline --release $ARGS \
     && cargo test --offline --release $ARGS -- --test-threads=1; then
    echo "-- combo '$combo': PASS"
  else
    echo "-- combo '$combo': FAIL"
    STATUS=1
  fi
done < "$COMBOS_FILE"
rm -f "$COMBOS_FILE"
exit $STATUS
