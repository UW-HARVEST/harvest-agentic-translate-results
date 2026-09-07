#!/usr/bin/env bash
# Full verification matrix: every cargo feature combination x every build
# variant of BOTH libraries. All comparisons go through the two .so files.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$W")"
fail=0

# --- feature combinations (extracted from Cargo.toml) ----------------------
feats=$(python3 - "$W/Cargo.toml" <<'PY'
import sys, itertools, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n not in ('default',):
                names.append(n)
print(len(names))
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        print(','.join(c))
PY
)
nfeat=$(echo "$feats" | head -1)
echo "### feature axes declared in Cargo.toml: $nfeat"

# --- extra C build variants ------------------------------------------------
for opt in O0 O2 O3; do
  d="$ROOT/c_src/build-$opt"
  mkdir -p "$d"
  ( cd "$d" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DCMAKE_C_FLAGS="-$opt" >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1 ) \
    || echo "  (could not build C variant -$opt)"
done

run_matrix() { # $1 = feature flags for cargo
  local fflags="$1"
  for rprof in release debug; do
    if [ "$rprof" = release ]; then
      ( cd "$W" && cargo build --release --offline $fflags >/dev/null 2>&1 )
      rso="$W/target/release/libnormalize_lib.so"
    else
      ( cd "$W" && cargo build --offline $fflags >/dev/null 2>&1 )
      rso="$W/target/debug/libnormalize_lib.so"
    fi
    [ -f "$rso" ] || { echo "  MISSING rust .so for $rprof"; fail=1; continue; }
    for copt in O0 O2 O3; do
      cso=$(ls "$ROOT/c_src/build-$copt"/*.so 2>/dev/null | head -1)
      [ -n "$cso" ] || continue
      label="feats=[${fflags:-default}] rust=$rprof c=-$copt"
      out=$( cd "$W" && DIFF_RUST_SO="$rso" DIFF_C_SO="$cso" \
             cargo test --offline --release $fflags -- --test-threads=4 2>&1 )
      if echo "$out" | grep -q 'test result: FAILED'; then
        echo "  FAIL  $label"
        echo "$out" | grep '\.\.\. FAILED' | sed 's/^/        /'
        fail=1
      else
        n=$(echo "$out" | grep -oP 'test result: ok\. \K[0-9]+' | paste -sd+ | python3 -c "import sys;print(eval(sys.stdin.read().strip() or 0))")
        echo "  ok    $label   ($n assertions/tests passed)"
      fi
    done
  done
}

combos=$(echo "$feats" | tail -n +2)
echo "--- combination: <default> ---";               run_matrix ""
echo "--- combination: --no-default-features ---";   run_matrix "--no-default-features"
for combo in $combos; do
  [ -z "$combo" ] && continue
  echo "--- combination: --features $combo ---"
  run_matrix "--no-default-features --features $combo"
done

echo "--- combination: --all-features ---"; run_matrix "--all-features"

# Restore the canonical C build for the default harness path.
echo "### done"
