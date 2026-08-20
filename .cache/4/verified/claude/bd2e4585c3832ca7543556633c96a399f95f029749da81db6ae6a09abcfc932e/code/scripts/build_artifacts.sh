#!/bin/bash
# Build, for every (OP, REPEAT) configuration:
#   artifacts/<op>_<r>/libcdriver.so   -- C shared library from c_src/src/mdcore.c
#   artifacts/<op>_<r>/cbin/driver     -- C executable (cmake, src/mdcore.c + src/mdmain.c)
#   artifacts/<op>_<r>/libdriver.so    -- Rust cdylib
#   artifacts/<op>_<r>/rbin/driver     -- Rust executable (same basename so argv[0] matches)
# Nothing is written inside c_src/.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1
CFLAGS_COMMON="-fPIC -Wall"
combos="${1:-}"
if [ -z "$combos" ]; then combos="$(./scripts/combos.sh)"; fi

for c in $combos; do
  op="${c%,*}"; r="${c#*,}"
  d="$ROOT/artifacts/${op}_${r}"
  mkdir -p "$d"
  # --- C shared library (library half only: mdcore.c) ---
  gcc $CFLAGS_COMMON -shared -DOP="$op" -DREPEAT="$r" \
      -o "$d/libcdriver.so" c_src/src/mdcore.c || { echo "FAIL cc so $c"; exit 1; }
  # --- C executable via CMake, out-of-source ---
  bd="$ROOT/cbuild/${op}_${r}"
  if [ ! -x "$bd/driver" ]; then
    cmake -S c_src -B "$bd" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DOP="$op" -DREPEAT="$r" >/dev/null 2>&1 || { echo "FAIL cmake cfg $c"; exit 1; }
    cmake --build "$bd" >/dev/null 2>&1 || { echo "FAIL cmake build $c"; exit 1; }
  fi
  mkdir -p "$d/cbin"; cp "$bd/driver" "$d/cbin/driver"
  # --- Rust cdylib + bin ---
  cargo build --offline --quiet --no-default-features --features "$op,$r" \
      || { echo "FAIL cargo $c"; exit 1; }
  cp target/debug/libdriver.so "$d/libdriver.so"
  mkdir -p "$d/rbin"; cp target/debug/driver "$d/rbin/driver"
  echo "built $op REPEAT=$r"
done
