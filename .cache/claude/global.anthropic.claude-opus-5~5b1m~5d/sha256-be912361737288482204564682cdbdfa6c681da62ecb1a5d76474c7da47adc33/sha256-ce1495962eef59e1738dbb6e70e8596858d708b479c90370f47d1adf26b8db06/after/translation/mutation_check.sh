#!/usr/bin/env bash
# Harness self-validation: inject a deliberate bug into src/lib.rs, rebuild the
# Rust cdylib, and confirm the differential suite FAILS. A test suite that
# cannot detect an injected bug proves nothing, so this is what justifies the
# "verified" claim.
#
# src/lib.rs is restored after every mutant. c_src/ is never touched.
set -u
cd "$(dirname "$0")" || exit 1

ORIG=$(mktemp) ; cp -p src/lib.rs "$ORIG"
restore() { cp -p "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

pass=0; fail=0

# cargo may satisfy a build from its cache by hard-linking an artifact whose
# mtime predates src/lib.rs. The suite's staleness guard would then fire even
# though the .so is current, so stamp the artifacts right after a build that
# cargo has confirmed up to date.
stamp() { touch target/debug/libdriver.so target/release/libdriver.so 2>/dev/null; true; }

mutate() {  # <name> <sed-expr> <expected-failing-test-substring>
  local name="$1" expr="$2" want="$3"
  restore
  perl -0pi -e "$expr" src/lib.rs
  if cmp -s "$ORIG" src/lib.rs; then
    echo "  !! MUTANT NOT APPLIED: $name  (pattern did not match)"; fail=$((fail+1)); return
  fi
  if ! cargo build --offline -q 2>/dev/null || ! cargo build --offline -q --release 2>/dev/null; then
    # A mutant that does not compile proves nothing about the test suite, and
    # must never be silently scored as "survived".
    echo "  !! MUTANT DID NOT COMPILE: $name  (fix the mutation, it tests nothing)"
    fail=$((fail+1)); return
  fi
  stamp
  out=$(timeout 600 cargo test --offline -q 2>&1)
  if ! echo "$out" | grep -q "test result:"; then
    echo "  !! TEST BINARY DID NOT RUN: $name"; fail=$((fail+1)); return
  fi
  if echo "$out" | grep -qE '^test result: FAILED|failures:'; then
    if [ -n "$want" ] && ! echo "$out" | grep -q "$want"; then
      echo "  ~  CAUGHT (but not by expected test '$want'): $name"
      echo "$out" | grep -E '^\s+\w+ ' | head -5
    else
      echo "  OK CAUGHT: $name"
    fi
    pass=$((pass+1))
  else
    echo "  !! SURVIVED (suite blind to this bug): $name"
    fail=$((fail+1))
  fi
}

echo "=== mutation testing the differential suite ==="

# 1. driver(): truthiness on the low byte only -> 0x100 wrongly reaches bad().
mutate "driver: useGood as u8 != 0 (byte-truncation)" \
  's/if useGood != 0 \{/if useGood as u8 != 0 {/' \
  "err_09_driver_out_of_range_int_values"

# 2. driver(): truthiness on the low 16 bits only.
mutate "driver: useGood as u16 != 0 (halfword-truncation)" \
  's/if useGood != 0 \{/if useGood as u16 != 0 {/' \
  "err_09_driver_out_of_range_int_values"

# 3. driver(): branches swapped.
mutate "driver: inverted condition" \
  's/if useGood != 0 \{/if useGood == 0 {/' \
  "cfg_21_27_driver_truthy_shapes"

# 4. good(): wrong constant.
mutate "good: writes 6 instead of 5" \
  's/data\.write\(5\)/data.write(6)/' \
  "cfg_19_good_single_call"

# 5. load_c_int(): reads 8 bytes instead of 4 (wrong element width). This must
#    be caught by the page-end row, where an 8-byte read runs off the mapping.
mutate "load_c_int: reads 8 bytes instead of 4 (wrong element width)" \
  's/"mov \{val:e\}, dword ptr \[\{ptr\}\]"/"mov {val:e}, dword ptr [{ptr} + 4]\\n mov {val:e}, dword ptr [{ptr}]"/' \
  "cfg_17_print_page_end_boundary"

# 6. printIntPtrLine(): big-endian byte swap of the loaded value.
mutate "printIntPtrLine: byte-swapped value" \
  's/printf\(FMT_D_NL\.as_ptr\(\), load_c_int\(intNumber\)\)/printf(FMT_D_NL.as_ptr(), load_c_int(intNumber).swap_bytes())/' \
  "cfg_01_06_print_int_ptr_line_value_traps"

# 6b. load_c_int(): off-by-one-element read (reads *(p+1) instead of *p).
mutate "load_c_int: off-by-one element (reads p+1)" \
  's/"mov \{val:e\}, dword ptr \[\{ptr\}\]"/"mov {val:e}, dword ptr [{ptr} + 4]"/' \
  "cfg_01_06_print_int_ptr_line_value_traps"

# 7. printIntPtrLine(): a "helpful" null check -- the classic well-meaning fix
#    that changes the C's observable behaviour (SIGSEGV) into a silent return.
mutate "printIntPtrLine: added null guard (behaviour change)" \
  's/printf\(FMT_D_NL\.as_ptr\(\), load_c_int\(intNumber\)\);/if intNumber.is_null() { return; }\n        printf(FMT_D_NL.as_ptr(), load_c_int(intNumber));/' \
  "err_01_print_int_ptr_line_null"

# 8. bad(): the CWE-457 defect "fixed" to a defined value -- must be rejected.
mutate "bad: CWE-457 sanitized to a defined 0" \
  's/let data_val: \*const c_int = unsafe \{ core::ptr::read_volatile\(data\.as_ptr\(\)\) \};/let zero: c_int = 0; let data_val: *const c_int = \&zero;/' \
  "err_07_bad_is_undefined_behaviour"

# 9. bad(): defect "fixed" by making it a null pointer (always SIGSEGV).
mutate "bad: CWE-457 sanitized to null (always faults)" \
  's/let data_val: \*const c_int = unsafe \{ core::ptr::read_volatile\(data\.as_ptr\(\)\) \};/let data_val: *const c_int = core::ptr::null();/' \
  "err_07_bad_is_undefined_behaviour"

# 10. bad(): silently replaced by good()'s behaviour.
mutate "bad: replaced by good()" \
  's/let data: MaybeUninit<\*const c_int> = MaybeUninit::uninit\(\);/unsafe { good() } return;\n    #[allow(unreachable_code)] let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();/' \
  "err_08_driver_zero_dispatches_to_bad"

# 11. printIntPtrLine: not exported (symbol parity must catch it).
mutate "printIntPtrLine: #[no_mangle] removed" \
  's/#\[unsafe\(no_mangle\)\]\npub unsafe extern "C" fn printIntPtrLine/pub unsafe extern "C" fn printIntPtrLine/' \
  "symbols_00_nm_parity"

# --- UB-check robustness -------------------------------------------------
# The plain Rust deref `*intNumber` diverges from C only when rustc's UB-checks
# are enabled: they panic, and a panic in an `extern "C"` fn aborts, so the
# library dies with SIGABRT where the C faults with SIGSEGV (or succeeds, for a
# legal misaligned read). `Cargo.toml` disables debug-assertions for the dev
# profile, but that alone is fragile -- anyone building with
# `-C debug-assertions=yes` would silently get different behaviour. The inline-asm
# load in `load_c_int` is what makes the translation profile-INDEPENDENT.
#
# Prove both halves of that claim, with UB-checks FORCED ON:
#   (a) the real translation still passes;
#   (b) the plain-deref version fails.
echo
echo "=== UB-check robustness (RUSTFLAGS=-C debug-assertions=yes) ==="
export RUSTFLAGS="-C debug-assertions=yes"

restore
cargo build --offline -q 2>/dev/null; cargo build --offline -q --release 2>/dev/null; stamp
out=$(timeout 600 cargo test --offline -q 2>&1)
if echo "$out" | grep -qE '^test result: FAILED|failures:'; then
  echo "  !! REGRESSION: the real translation FAILS with debug-assertions forced on"
  echo "$out" | grep -E '^\s+(cfg|err|symbols)_' | head -5
  fail=$((fail+1))
else
  echo "  OK (a) real translation passes with UB-checks forced on"
  pass=$((pass+1))
fi

perl -0pi -e 's/printf\(FMT_D_NL\.as_ptr\(\), load_c_int\(intNumber\)\)/printf(FMT_D_NL.as_ptr(), *intNumber)/' src/lib.rs
if cmp -s "$ORIG" src/lib.rs; then
  echo "  !! MUTANT NOT APPLIED: plain-deref"; fail=$((fail+1))
else
  cargo build --offline -q 2>/dev/null; cargo build --offline -q --release 2>/dev/null; stamp
  out=$(timeout 600 cargo test --offline -q 2>&1)
  if echo "$out" | grep -qE '^test result: FAILED|failures:'; then
    echo "  OK (b) plain-deref mutant CAUGHT with UB-checks forced on"
    pass=$((pass+1))
  else
    echo "  !! SURVIVED: plain-deref mutant not caught with UB-checks on"
    fail=$((fail+1))
  fi
fi

unset RUSTFLAGS
restore
cargo build --offline -q 2>/dev/null
cargo build --offline -q --release 2>/dev/null
stamp

echo
echo "=== checks passed: $pass   survived/errored: $fail ==="
[ "$fail" -eq 0 ] || exit 1
