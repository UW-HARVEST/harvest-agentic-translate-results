#!/bin/bash
# Negative control for the differential test suite.
#
# Matching symbols and green tests are only meaningful if the tests can actually
# FAIL. Each mutant below applies a one-line change to a COPY of
# translation/src/lib.rs, builds it into its own .so, and injects it via $RUST_SO.
# Every mutant must be caught by at least one test; a mutant that survives is a
# blind spot in the suite, not a harmless change.
#
# Nothing in c_src/ and nothing in translation/src/ is modified.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
WORK="${TMPDIR:-/tmp}/mutants"
rm -rf "$WORK"; mkdir -p "$WORK"
cp -r "$ROOT/translation" "$WORK/crate"
rm -rf "$WORK/crate/target" "$WORK/crate/tests"

apply() { # $1=name $2=sed-expr
  cp "$ROOT/translation/src/lib.rs" "$WORK/crate/src/lib.rs"
  sed -i "$2" "$WORK/crate/src/lib.rs" || return 1
  if cmp -s "$ROOT/translation/src/lib.rs" "$WORK/crate/src/lib.rs"; then
    echo "sed expression matched nothing -- fix the mutant definition"; return 1
  fi
  ( cd "$WORK/crate" && cargo build --release -q 2>&1 | tail -5 )
}

names=(); exprs=()
add() { names+=("$1"); exprs+=("$2"); }

#   name                            one-line mutation
add no-wrap-overflow                's/x.wrapping_mul(2)/x.saturating_mul(2)/'
add forward-returns-x-not-2x        's/x.wrapping_mul(2)/x/'
add negative-boundary-off-by-one    's/if x < 0 {/if x <= 0 {/'
add wrong-stderr-text               's/Error: negative input/Error: Negative input/'
add processing-msg-spacing          's/Processing: %d/Processing:  %d/'
add buffer-size-128                 's/\[0 as c_char; 100\]/[0 as c_char; 128]/'
add fgets-len-off-by-one            's/buffer.len() as c_int/(buffer.len() - 1) as c_int/'
add loop-printf-emits-nothing       's/b"%s\\0"/b"\\0"/'
add cleanup-skips-fclose            's/^        fclose(fp);$/        let _ = fp;/'
add ferror-check-removed            's/if ferror(fp) != 0 {/if false {/'
add driver-wrong-sentinel           's/^        return -2;$/        return -1;/'
add driver-res-check-wrong          's/if res == -1 {/if res == -2 {/'
add driver-missing-goto-output      's/Goto output: %d/Goto output:%d/'
add fopen-mode-rplus                's/b"r\\0"/b"r+\\0"/'
add open-fail-msg-text              's/Error: opening or processing file/Error: opening or processing FILE/'

pass=0; fail=0; survivors=()
for i in "${!names[@]}"; do
  n="${names[$i]}"; e="${exprs[$i]}"
  printf '=== %-32s ' "$n"
  if ! apply "$n" "$e" >"$WORK/build-$n.log" 2>&1; then
    echo "BUILD/SED FAILED (see $WORK/build-$n.log)"; fail=$((fail+1)); survivors+=("$n(build)"); continue
  fi
  out=$(cd "$ROOT/translation" && RUST_SO="$WORK/crate/target/release/libdriver.so" \
        timeout 300 cargo test 2>&1)
  nf=$(printf '%s\n' "$out" | grep -cE '^test .* \.\.\. FAILED')
  if [ "$nf" -gt 0 ]; then
    echo "CAUGHT by $nf test(s)"; pass=$((pass+1))
  else
    echo "*** SURVIVED -- blind spot in the test suite ***"
    printf '%s\n' "$out" | tail -15
    fail=$((fail+1)); survivors+=("$n")
  fi
done

echo
echo "mutants caught: $pass / $((pass+fail))"
if [ "$fail" -ne 0 ]; then
  echo "survivors: ${survivors[*]}"
  exit 1
fi
echo "negative control PASSED: every mutant is detected"
