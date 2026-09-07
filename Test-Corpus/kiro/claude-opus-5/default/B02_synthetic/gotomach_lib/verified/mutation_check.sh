#!/usr/bin/env bash
# Mutation test: inject a known bug into the Rust translation and confirm the
# differential suite FAILS. A mutation that survives means the suite has a blind
# spot. The original src/lib.rs is always restored.
set -u
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

declare -a NAMES=() FROM=() TO=()
add() { NAMES+=("$1"); FROM+=("$2"); TO+=("$3"); }

add "process_value off-by-one"      'value.wrapping_add(10)'                'value.wrapping_add(11)'
add "double_value wrong factor"     'value.wrapping_mul(2)'                 'value.wrapping_mul(3)'
add "triple_value wrong factor"     'value.wrapping_mul(3)'                 'value.wrapping_mul(4)'
add "iterations bound >= not >"     'iterations > UINT16_MAX'               'iterations >= UINT16_MAX'
add "seed bound >= not >"           'seed > UINT16_MAX'                     'seed >= UINT16_MAX'
add "iterations lower bound <="     'iterations < 0 ||'                     'iterations <= 0 ||'
add "wrong iteration err code"      'result = -1;'                          'result = -7;'
add "wrong seed err code"           'result = -2;'                          'result = -1;'
add "range check order swapped"     'if iterations < 0 || iterations > UINT16_MAX' 'if seed < 0 || seed > UINT16_MAX'
add "threshold <= not <"            'if produced < threshold'               'if produced <= threshold'
add "modulus 1000 -> 1001"          'produced.wrapping_rem(1000)'           'produced.wrapping_rem(1001)'
add "default mode -> triple"        '_ => {
                LOG_MSG!(WARNING, "Invalid mode, using default");
                selected_op = Some(process_value);' '_ => {
                LOG_MSG!(WARNING, "Invalid mode, using default");
                selected_op = Some(triple_value);'
add "mode 1/2 swapped"              '1 => selected_op = Some(double_value),' '1 => selected_op = Some(triple_value),'
add "max-count cutoff off-by-one"   '(*state).count >= UINT16_MAX as usize' '(*state).count > UINT16_MAX as usize'
add "dropped INFO start log"        'LOG_MSG!(INFO, "Starting gotomach function");' '{}'
add "dropped WARNING mode log"      'LOG_MSG!(WARNING, "Invalid mode, using default");' '{}'
add "dropped success log"           'LOG_MSG!(INFO, "Processing completed successfully");' '{}'
add "wrong error message text"      'LOG_MSG!(ERROR, "Invalid seed value");' 'LOG_MSG!(ERROR, "Invalid seed valu");'
add "sum skips first element"       'let mut j: usize = 0;'                 'let mut j: usize = 1;'

pass=0; caught=0; survived=0; skipped=0
printf '%-34s %s\n' "MUTATION" "RESULT"
printf '%-34s %s\n' "----------------------------------" "------"
for i in "${!NAMES[@]}"; do
  restore
  python3 - "$ORIG" src/lib.rs "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
src, dst, a, b = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
t = open(src).read()
if t.count(a) < 1:
    sys.exit(2)
open(dst, 'w').write(t.replace(a, b, 1))
PY
  rc=$?
  if [ $rc -ne 0 ]; then
    printf '%-34s %s\n' "${NAMES[$i]}" "SKIPPED (pattern not found)"
    skipped=$((skipped+1)); continue
  fi
  if ! cargo build --release >/dev/null 2>&1; then
    printf '%-34s %s\n' "${NAMES[$i]}" "SKIPPED (does not compile)"
    skipped=$((skipped+1)); continue
  fi
  if timeout 600 cargo test --release --no-fail-fast -- --test-threads=1 >/tmp/mut.log 2>&1; then
    printf '%-34s %s\n' "${NAMES[$i]}" "*** SURVIVED (blind spot!) ***"
    survived=$((survived+1))
  else
    n=$(grep -c '^test .* FAILED' /tmp/mut.log)
    printf '%-34s %s\n' "${NAMES[$i]}" "caught ($n failing tests)"
    caught=$((caught+1))
  fi
done

restore
cargo build --release >/dev/null 2>&1
echo
echo "caught=$caught survived=$survived skipped=$skipped"
[ "$survived" -eq 0 ] || exit 1
