#!/usr/bin/env bash
# Non-vacuity check for the differential suite.
#
# Injects a deliberate bug into src/lib.rs, rebuilds the cdylib, and asserts
# that the differential tests FAIL. A mutation that survives means the suite
# has a blind spot at that branch.
#
# src/lib.rs is always restored, including on Ctrl-C.
set -uo pipefail
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; rm -f "$ORIG"; }
trap restore EXIT INT TERM

# Patterns are anchored to real CODE (e.g. inside `c_print(b"...")`) so that a
# match in a doc comment cannot masquerade as a mutation.
declare -a MUTS=(
  's/result = 1;/result = 9;/|status code for the x-check'
  's/result = 2;/result = 0;/|status code for the y-check'
  's/result = 3;/result = 4;/|status code for the z-check'
  's/if x != 1 \{/if x != 2 {/|the x comparison constant'
  's/Y\.load\(Ordering::Relaxed\) != 2/Y.load(Ordering::Relaxed) != 3/|the y comparison constant'
  's/if z != 3 \{/if z != 4 {/|the z comparison constant'
  's/c_print\(b"Error: x != 1/c_print(b"Error: x !=1/|the x-check message text'
  's/c_print\(b"Error: x == 1 but y != 2/c_print(b"Error: x == 1 but y != 20/|the y-check message text'
  's/c_print\(b"Error: x == 1 and y == 2, but z != 3/c_print(b"Error: x == 1 and y == 2 but z != 3/|the z-check message text'
  's/c_print\(b"Operation failed/c_print(b"Operation Failed/|the shared fail-epilogue text'
  's/c_print\(b"Ok!/c_print(b"OK!/|the success message text'
  's/c"Result: %d\\n"/c"Result: %d"/|the trailing newline of the Result line'
  's/c"Result: %d\\n"/c"result: %d\\n"/|the case of the Result label'
  's/Y\.store\(local_y, Ordering::Relaxed\);//|the write of local_y into the static'
  's/let mut result: c_int = 0;/let mut result: c_int = 7;/|the initial value of result'
  # EQUIVALENT MUTANT (expected to survive, and correct that it does):
  # `driver` unconditionally does `Y.store(local_y)` before `multi_stage` ever
  # reads `Y`, and `Y`/`y` is not exported, so the initialiser is dead code in
  # BOTH implementations. No input to the public API can distinguish it.
  # C: `static int y = 123;` -- likewise never read before being overwritten.
  's/AtomicI32::new\(123\)/AtomicI32::new(2)/|EQUIVALENT:the initial value of the static y (dead in C too)'
  's/c_print\(b"Operation failed\\n\\0"\);\n    result/result/|dropping the fail epilogue entirely'
  's/let result = multi_stage\(x, z\);/let result = multi_stage(z, x);/|the argument order passed to multi_stage'
)

pass=0; survived=(); equivalent=()
for entry in "${MUTS[@]}"; do
  expr="${entry%%|*}"; desc="${entry#*|}"
  # A leading `EQUIVALENT:` marks a mutant that provably cannot be observed
  # through the public API (documented at its definition above).
  expect_survive=0
  case "$desc" in
    EQUIVALENT:*) expect_survive=1; desc="${desc#EQUIVALENT:}" ;;
  esac
  cp "$ORIG" src/lib.rs
  if ! perl -0pi -e "$expr" src/lib.rs; then
    echo "  SKIP (perl failed): $desc"; continue
  fi
  if cmp -s "$ORIG" src/lib.rs; then
    echo "  SKIP (pattern did not match): $desc"; survived+=("$desc [pattern unmatched]"); continue
  fi

  printf '  mutating %-50s ... ' "$desc"
  if ! timeout 300 cargo build --release --offline >/dev/null 2>&1; then
    echo "caught (does not compile)"; pass=$((pass+1)); continue
  fi
  if timeout 300 cargo test --release --offline >/dev/null 2>&1; then
    if [ "$expect_survive" -eq 1 ]; then
      echo "survived (expected: equivalent mutant)"; equivalent+=("$desc"); pass=$((pass+1))
    else
      echo "SURVIVED  <-- blind spot!"; survived+=("$desc")
    fi
  else
    if [ "$expect_survive" -eq 1 ]; then
      echo "caught (unexpectedly - the mutant is NOT equivalent after all)"; pass=$((pass+1))
    else
      echo "caught"; pass=$((pass+1))
    fi
  fi
done

cp "$ORIG" src/lib.rs
timeout 300 cargo build --release --offline >/dev/null 2>&1

echo
echo "mutations accounted for: $pass / ${#MUTS[@]}"
if [ "${#equivalent[@]}" -ne 0 ]; then
  echo "equivalent mutants (unobservable through the public API, by design):"
  printf '  - %s\n' "${equivalent[@]}"
fi
if [ "${#survived[@]}" -ne 0 ]; then
  echo "SURVIVING MUTATIONS (test-suite blind spots):"
  printf '  - %s\n' "${survived[@]}"
  exit 1
fi
echo "No unexpected survivors: the differential suite covers every observable branch."
