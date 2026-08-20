#!/usr/bin/env bash
# Full verification driver: builds the C reference, enumerates every cargo
# feature combination, and for each one runs cargo check, builds the cdylib,
# diffs the exported symbols against the C .so, and runs Phases B/C/D.
#
# Usage: ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")"
FAILED=0
LOG="${TMPDIR:-/tmp}/verify_logs"
mkdir -p "$LOG"
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- C reference
step "Building the C reference shared library"
if (mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null) 2>&1 | tail -3; then
  ok "c_src/build/libtranslated_rust.so"
else
  bad "C build"; exit 1
fi
C_SO=c_src/build/libtranslated_rust.so

# ------------------------------------------------- enumerate feature combinations
step "Enumerating feature combinations"
python3 - <<'PY'
import itertools, pathlib, re
s = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                feats.append(n)
combos = [",".join(c) for r in range(len(feats) + 1)
          for c in itertools.combinations(feats, r)]
pathlib.Path("feature_combos.txt").write_text("\n".join(combos) + "\n")
print(f"   features: {feats or '(none declared)'}")
print(f"   combinations: {len(combos)}")
PY
mapfile -t COMBOS < feature_combos.txt
ok "${#COMBOS[@]} combination(s)"

# --------------------------------------------------------- per-combination work
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  step "Combination: $label"

  if timeout 600 cargo check --no-default-features --features "$combo" \
       >"$LOG/chk.log" 2>&1; then
    ok "cargo check"
  else
    bad "cargo check"; tail -20 "$LOG/chk.log"; continue
  fi

  if timeout 600 cargo build --no-default-features --features "$combo" \
       >"$LOG/bld.log" 2>&1; then
    ok "cargo build (cdylib)"
  else
    bad "cargo build"; tail -20 "$LOG/bld.log"; continue
  fi

  # Symbol parity: prefer the freshly-written deps/ copy.
  R_SO=$(ls -t target/debug/deps/libcleanup_lib.so target/debug/libcleanup_lib.so \
           2>/dev/null | head -1)
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
    <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort))
  if [ -z "$missing" ]; then
    ok "symbol parity ($(nm -D --defined-only "$C_SO" | wc -l) C symbols, 0 missing)"
  else
    bad "symbols missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  fi

  if timeout 600 cargo test --no-default-features --features "$combo" -- --nocapture \
       >"$LOG/tst.log" 2>&1; then
    ok "Phases B/C/D: $(grep -c 'test result: ok' "$LOG/tst.log") test target(s) green"
    grep -E '^  [BEG][0-9]+.*\.\.\. ' "$LOG/tst.log" | wc -l \
      | xargs printf '        %s differential rows executed\n'
  else
    bad "cargo test"; grep -E 'FAILED|panicked|DIVERGENCE' "$LOG/tst.log" | head -20
  fi
done

# ------------------------------------------------------ default-features sanity
step "Default feature set (plain \`cargo test\`)"
if timeout 600 cargo test >"$LOG/def.log" 2>&1; then
  ok "cargo test (default features)"
else
  bad "cargo test (default features)"; grep -E 'FAILED|panicked' "$LOG/def.log" | head
fi

# ------------------------------------------------------------- release profile
step "Release profile (optimized Rust .so vs the same C .so)"
if timeout 600 cargo test --release --no-default-features >"$LOG/rel.log" 2>&1; then
  ok "cargo test --release"
  inc=$(grep -c 'INCONCLUSIVE' "$LOG/rel.log" || true)
  [ "$inc" -gt 0 ] && printf '        %s leak-probe row(s) self-reported inconclusive (expected under --release)\n' "$inc"
else
  bad "cargo test --release"; grep -E 'FAILED|panicked|SIGABRT' "$LOG/rel.log" | head
fi

# --------------------------------------------------------- anti-vacuity check
step "Mutation check (proves the suite can detect divergence)"
if timeout 600 python3 mutation_check.py >"$LOG/mut.log" 2>&1; then
  ok "$(grep -oE 'caught [0-9]+/[0-9]+.*' "$LOG/mut.log" | head -1)"
else
  bad "surviving mutations"; sed -n '/SURVIVING/,$p' "$LOG/mut.log"
fi

step "RESULT"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAILED"
