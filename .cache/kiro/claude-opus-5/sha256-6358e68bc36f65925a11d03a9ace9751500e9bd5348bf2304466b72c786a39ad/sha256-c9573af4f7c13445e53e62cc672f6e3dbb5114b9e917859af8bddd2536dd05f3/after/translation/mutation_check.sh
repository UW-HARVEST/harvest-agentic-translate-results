#!/usr/bin/env bash
# Mutation check: proves the differential suite actually detects divergence.
#
# For each mutant, a copy of the crate is patched with one plausible
# translation bug, built as a cdylib, and the *unmodified* test suite is
# pointed at it via POW_RUST_SO. Every mutant MUST make the suite fail; a
# surviving mutant means the tests are not really checking that behaviour.
set -uo pipefail

CRATE="$(cd "$(dirname "$0")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Mutants: name | sed program applied to src/pow.rs
declare -a NAMES=(
  "no_errno_reset"
  "skip_edom_branch"
  "skip_erange_branch"
  "return_plus_one"
  "swap_edom_erange_order"
  "wrong_edom_value"
  "wrong_erange_value"
  "errno_read_before_pow"
  "return_result_on_edom"
  "typo_in_message"
  "swap_printed_args"
  "wrong_precision"
  "no_trailing_newline"
)
declare -a SEDS=(
  's/ffi::set_errno(0);//'
  's/if ffi::errno() == EDOM {/if false {/'
  's/} else if ffi::errno() == ERANGE {/} else if false {/'
  's/return -1\.0;/return 1.0;/g'
  's/== EDOM/== ERANGE/; s/} else if ffi::errno() == ERANGE {/} else if ffi::errno() == EDOM {/'
  's/== EDOM/== 1/'
  's/== ERANGE/== 1/'
  's/ffi::set_errno(0);/ffi::set_errno(0); let _pre = ffi::errno();/; s/if ffi::errno() == EDOM {/if _pre == EDOM {/'
  's/return -1\.0;/return result;/'
  's/Domain error:/Domain Error:/'
  's/^                base,$/                exponent,/; s/^                exponent,$/                base,/'
  's/%\.2f/%.3f/g'
  's/domain\.\\n"/domain."/'
)

# Mutants known to be behaviourally EQUIVALENT on Linux/glibc, so surviving is
# correct and not a test gap. Documented in ERRORS.md.


pass=0
fail=0
for i in "${!NAMES[@]}"; do
  name="${NAMES[$i]}"
  dir="$WORK/$name"
  mkdir -p "$dir"
  cp -r "$CRATE/src" "$CRATE/Cargo.toml" "$dir/"
  # Mutant crates are plain cdylibs; no dev-deps needed.
  sed -i 's/^\[dev-dependencies\]/[dev-dependencies-disabled]/' "$dir/Cargo.toml"
  sed -i "${SEDS[$i]}" "$dir/src/pow.rs"

  if ! (cd "$dir" && timeout 300 cargo build --quiet >/dev/null 2>&1); then
    echo "SKIP  $name (mutant does not compile)"
    continue
  fi
  so="$dir/target/debug/libpow.so"
  [ -f "$so" ] || so="$dir/target/debug/deps/libpow.so"
  if [ ! -f "$so" ]; then
    echo "SKIP  $name (no cdylib produced)"
    continue
  fi

  if POW_RUST_SO="$so" timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" \
       --test phase_b_valid --test phase_c_errors --quiet >/dev/null 2>&1; then
    echo "SURVIVED  $name  <-- suite did NOT detect this bug"
    fail=$((fail+1))
  else
    echo "killed    $name"
    pass=$((pass+1))
  fi
done

echo "-----"
echo "mutants killed: $pass, survived: $fail"
[ "$fail" -eq 0 ]
