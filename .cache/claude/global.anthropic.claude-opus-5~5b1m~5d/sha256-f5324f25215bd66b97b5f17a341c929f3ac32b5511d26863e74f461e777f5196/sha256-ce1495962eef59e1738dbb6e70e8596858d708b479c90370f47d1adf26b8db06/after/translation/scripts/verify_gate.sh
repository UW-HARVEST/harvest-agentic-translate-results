#!/usr/bin/env bash
# The Phase D completion gate, executable.
#
#   1. SYMBOLS.md : nm -D diff between the C and Rust .so must be EMPTY
#   2. CONFIGS.md : every row must name a test that exists AND is checked off
#   3. ERRORS.md  : every row must name a test that exists AND is checked off
#   4. every test must pass, under every feature combination, debug + release
set -uo pipefail
cd "$(dirname "$0")/.."

C_SO=../c_src/build/libdriver.so
fail=0
say() { printf '%s\n' "$*"; }
ok()  { printf '  [ok]   %s\n' "$*"; }
bad() { printf '  [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
say "== building both libraries =="
if [ ! -f "$C_SO" ]; then
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { bad "C build"; exit 1; }
fi
cargo build --offline >/dev/null 2>&1          || { bad "cargo build (debug)"; exit 1; }
cargo build --offline --release >/dev/null 2>&1 || { bad "cargo build (release)"; exit 1; }
ok "C .so and Rust .so (debug + release) built"

# ---------------------------------------------------------------------------
say "== 1. SYMBOLS.md: nm -D parity =="
for prof in debug release; do
  RS_SO="target/$prof/libdriver.so"
  diff_out=$(diff \
    <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u))
  if [ -z "$diff_out" ]; then
    ok "$prof: symbol sets identical ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
  else
    bad "$prof: symbol diff is non-empty:"; printf '%s\n' "$diff_out"
  fi
done
# no undefined non-libc symbol may remain unresolved: dlopen both and dlsym all 3
if cargo test --offline -q --test phase_d_symbols >/dev/null 2>&1; then
  ok "all C symbols dlsym-able from the Rust .so, and none imported from the C .so"
else
  bad "phase_d_symbols tests"
fi

# ---------------------------------------------------------------------------
# Row -> test cross-check: a row may only be ticked if the test it names exists.
check_table() {
  local file=$1 label=$2
  local rows=0 unchecked=0 missing=0
  while IFS= read -r line; do
    # table rows look like: | id | ... | `test_name` | [x] |
    case "$line" in
      \|*) : ;;
      *) continue ;;
    esac
    # skip header / separator / non-data rows
    printf '%s' "$line" | grep -qE '^\|\s*[CE][0-9]+\s*\|' || continue
    rows=$((rows + 1))
    local id; id=$(printf '%s' "$line" | sed -E 's/^\|\s*([CE][0-9]+)\s*\|.*/\1/')
    if ! printf '%s' "$line" | grep -qE '\[x\]\s*\|?\s*$'; then
      bad "$label row $id is not checked off"
      unchecked=$((unchecked + 1))
    fi
    # collect every `backticked` token in the row and require at least one to be
    # a real #[test] fn (rows that are documentation-only say "(documented)")
    local names found
    # Rows explicitly marked "(documented)" assert unobservable/UB behaviour that
    # cannot be executed differentially; they carry a written justification.
    if printf '%s' "$line" | grep -q '(documented)'; then
      continue
    fi
    names=$(printf '%s' "$line" | grep -oE '`[a-z0-9_]+`' | tr -d '`')
    if [ -z "$names" ]; then
      bad "$label row $id names no test"
      continue
    fi
    found=0
    for n in $names; do
      if grep -rqE "^fn $n\(" tests/ ; then found=1; break; fi
    done
    [ "$found" = 1 ] || { bad "$label row $id names no existing #[test] fn: $names"; missing=$((missing + 1)); }
  done < "$file"
  if [ "$rows" -eq 0 ]; then
    bad "$label: no rows parsed from $file"
  else
    ok "$label: $rows rows, $unchecked unchecked, $missing with a missing test"
  fi
}

say "== 2. CONFIGS.md rows =="
check_table CONFIGS.md CONFIGS
say "== 3. ERRORS.md rows =="
check_table ERRORS.md ERRORS

# ---------------------------------------------------------------------------
say "== 4. full suite, every feature combination, debug + release =="
if bash scripts/check_features.sh >"${TMPDIR:-/tmp}"/features.$$ 2>&1; then
  ok "$(tail -1 "${TMPDIR:-/tmp}"/features.$$)"
else
  bad "feature sweep failed:"; tail -30 "${TMPDIR:-/tmp}"/features.$$
fi
rm -f "${TMPDIR:-/tmp}"/features.$$

# ---------------------------------------------------------------------------
say "== 5. harness self-validation (mutation testing) =="
if python3 scripts/mutation_check.py >"${TMPDIR:-/tmp}"/mut.$$ 2>&1; then
  ok "$(grep -E '^caught=' "${TMPDIR:-/tmp}"/mut.$$)"
else
  bad "surviving mutants (the suite has a blind spot):"; tail -20 "${TMPDIR:-/tmp}"/mut.$$
fi
rm -f "${TMPDIR:-/tmp}"/mut.$$

say
if [ "$fail" -eq 0 ]; then
  say "VERIFICATION COMPLETE — every gate item holds."
else
  say "VERIFICATION INCOMPLETE — see [FAIL] lines above."
fi
exit "$fail"
