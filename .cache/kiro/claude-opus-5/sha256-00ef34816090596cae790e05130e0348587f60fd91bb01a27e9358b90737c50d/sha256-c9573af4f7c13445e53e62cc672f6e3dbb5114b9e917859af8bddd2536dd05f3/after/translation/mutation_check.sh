#!/usr/bin/env bash
# Mutation check: prove the differential suite is not vacuously passing.
#
# Injects each mutation into translation/src/lib.rs, runs `cargo test`, and
# requires the suite to FAIL. Restores the original source afterwards.
# c_src/ is never touched.
set -u

cd "$(dirname "$0")" || exit 1
SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

# name<TAB>expected<TAB>sed expression
#   expected=kill       -> the suite MUST fail (an observable divergence)
#   expected=equivalent -> provably unobservable through the C ABI; the suite
#                          SHOULD still pass. `smallestValue` returns only the
#                          minimum VALUE, never which node produced it, so
#                          updating the running minimum on ties (`<=` instead of
#                          `<`) stores an identical integer and cannot be
#                          distinguished by any caller. Recorded here so the
#                          reason is explicit rather than looking like a gap.
mutations=$(cat <<'EOF'
strict-lt-becomes-le	equivalent	s/if node\.value < smallest/if node.value <= smallest/
null-sentinel-0	kill	s/^        -1$/        0/
null-sentinel-neg2	kill	s/^        -1$/        -2/
skip-head-value	kill	s/let mut smallest: c_int = node\.value;/let mut smallest: c_int = c_int::MAX;/
off-by-one-skip-last	kill	s/while !node\.next\.is_null()/while !node.next.is_null() \&\& !unsafe { \&*node.next }.next.is_null()/
negate-comparison	kill	s/if node\.value < smallest/if node.value > smallest/
seed-from-second	kill	s/let mut node: &ListNode = head;/let mut node: \&ListNode = if head.next.is_null() { head } else { unsafe { \&*head.next } };/
EOF
)

fail=0
while IFS=$'\t' read -r name expected expr; do
  [ -z "$name" ] && continue
  cp "$BAK" "$SRC"
  sed -i "$expr" "$SRC"
  if cmp -s "$BAK" "$SRC"; then
    echo "SETUP-ERROR   $name: mutation did not apply (pattern no longer matches)"
    fail=1
    continue
  fi
  if timeout 600 cargo test --quiet >/dev/null 2>&1; then
    passed=yes
  else
    passed=no
  fi

  case "$expected:$passed" in
    kill:no)        echo "KILLED        $name" ;;
    kill:yes)       echo "SURVIVED      $name  <-- suite failed to catch this; tests are too weak"; fail=1 ;;
    equivalent:yes) echo "EQUIVALENT    $name  (unobservable through the ABI, as documented)" ;;
    equivalent:no)  echo "UNEXPECTED    $name  (claimed equivalent but the suite caught it)"; fail=1 ;;
  esac
done <<< "$mutations"

cp "$BAK" "$SRC"
if timeout 600 cargo test --quiet >/dev/null 2>&1; then
  echo "RESTORED      original source passes the suite"
else
  echo "RESTORE-ERROR original source no longer passes"
  fail=1
fi

exit $fail
