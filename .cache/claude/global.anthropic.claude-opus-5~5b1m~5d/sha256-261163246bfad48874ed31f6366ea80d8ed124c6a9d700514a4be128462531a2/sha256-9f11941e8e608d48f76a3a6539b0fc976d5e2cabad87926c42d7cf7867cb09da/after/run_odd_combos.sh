#!/bin/bash
# Feature combinations that have no direct CMake counterpart: no OP feature at
# all, no REPEAT feature at all, and several OPs / REPEATs enabled together.
# src/mdconfig.rs resolves these with a documented priority (OP: add>sub>mul,
# falling back to the C default `add`; REPEAT: lowest requested wins, default 5).
# This script resolves the same way, builds the corresponding C config, and runs
# the full differential suite against it.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
LOGS="$ROOT/logs"; mkdir -p "$LOGS"

resolve_op () { case ",$1," in *,add,*) echo add;; *,sub,*) echo sub;; *,mul,*) echo mul;; *) echo add;; esac; }
resolve_rep () {
  for n in 0 1 2 3 4 5 6 7; do case ",$1," in *,$n,*) echo "$n"; return;; esac; done
  echo 5
}

COMBOS=(
  ""                    # no features at all -> add / 5 (both C #ifndef defaults)
  "add"                 # no REPEAT         -> add / 5
  "sub"                 # no REPEAT         -> sub / 5
  "mul"                 # no REPEAT         -> mul / 5
  "0" "3" "7"           # no OP             -> add / n
  "add,sub"             # OP priority       -> add / 5
  "sub,mul"             # OP priority       -> sub / 5
  "add,sub,mul"         # OP priority       -> add / 5
  "add,sub,mul,0,1,2,3,4,5,6,7"  # everything -> add / 0
  "mul,3,5"             # REPEAT priority   -> mul / 3
  "sub,6,7"             # REPEAT priority   -> sub / 6
  "mul,2,7"             # REPEAT priority   -> mul / 2
  "add,mul,4,6"         # both priorities   -> add / 4
)

pass=0; fail=0
for f in "${COMBOS[@]}"; do
  op=$(resolve_op "$f"); rep=$(resolve_rep "$f")
  tag="odd_$(echo "${f:-none}" | tr ',' '-')"
  log="$LOGS/$tag.log"
  {
    echo "########## FEATURES='$f' -> OP=$op REPEAT=$rep ##########"
    "$ROOT/build_c.sh" "$op" "$rep"                                     || exit 90
    cd "$ROOT/translation"                                              || exit 91
    timeout 300 cargo build --release --offline --no-default-features --features "$f" || exit 92
    nm -D --defined-only --format=posix "$ROOT/cbuild/${op}_${rep}/libdriver_c.so" | awk '{print $1}' | sort -u > "$LOGS/$tag.c.syms"
    nm -D --defined-only --format=posix "$ROOT/translation/target/release/libdriver.so" | awk '{print $1}' | sort -u > "$LOGS/$tag.rust.syms"
    missing=$(comm -23 "$LOGS/$tag.c.syms" "$LOGS/$tag.rust.syms")
    [ -n "$missing" ] && { echo "SYMBOL DIFF NOT EMPTY: $missing"; exit 93; }
    echo "symbol diff: EMPTY"
    timeout 500 cargo test --release --offline --no-default-features --features "$f" || exit 94
  } > "$log" 2>&1
  rc=$?
  if [ $rc -eq 0 ]; then
    printf 'PASS  features=%-30s -> %s/%s\n' "'${f:-<none>}'" "$op" "$rep"; pass=$((pass+1))
  else
    printf 'FAIL  features=%-30s -> %s/%s (rc=%d) see %s\n' "'${f:-<none>}'" "$op" "$rep" "$rc" "$log"; fail=$((fail+1))
  fi
done
echo
echo "odd/degenerate combos passed: $pass  failed: $fail"
exit $fail
