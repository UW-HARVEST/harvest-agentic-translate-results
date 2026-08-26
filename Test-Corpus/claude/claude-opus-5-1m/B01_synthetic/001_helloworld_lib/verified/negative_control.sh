#!/usr/bin/env bash
# Anti-vacuity check: point the test suite at deliberately WRONG "translations"
# and confirm the differential tests reject each of them. If a mutant passes,
# the corresponding test row proves nothing.
#
#   ./negative_control.sh
set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}/hello-mutants"
mkdir -p "$TMP"

cat > "$TMP/m1.c" <<'EOF'
#include <stdio.h>
int helloworld() { printf("Hello World!"); return 0; }              /* no '\n' */
EOF
cat > "$TMP/m2.c" <<'EOF'
#include <stdio.h>
int helloworld() { printf("Hello World!\n"); return 1; }            /* wrong return */
EOF
cat > "$TMP/m3.c" <<'EOF'
#include <stdio.h>
int helloworld_typo() { printf("Hello World!\n"); return 0; }       /* symbol missing */
EOF
cat > "$TMP/m4.c" <<'EOF'
#include <stdio.h>
static int n = 0;
int helloworld() { if (n++ == 0) printf("Hello World!\n"); return 0; } /* hidden state */
EOF
cat > "$TMP/m5.c" <<'EOF'
#include <stdio.h>
/* "helpfully" propagates the I/O error the real C code ignores: invisible to
   happy-path tests, caught only by the Phase C error rows. */
int helloworld() { if (printf("Hello World!\n") < 0) return -1; return 0; }
EOF

echo "building mutants in $TMP"
for m in m1 m2 m3 m4 m5; do
  gcc -shared -fPIC -o "$TMP/lib$m.so" "$TMP/$m.c" || { echo "compile failed: $m"; exit 1; }
done

BAD=0
for m in m1 m2 m3 m4 m5; do
  echo
  echo "################ mutant $m ################"
  sed -n '2,4p' "$TMP/$m.c"
  caught=0
  for t in phase_b phase_c phase_d; do
    out=$(HELLO_RUST_SO="$TMP/lib$m.so" HELLO_RUST_SO_RELEASE="$TMP/lib$m.so" \
          timeout 600 cargo test --offline --test "$t" -- --nocapture --test-threads=1 2>&1)
    rc=$?
    n=$(printf '%s\n' "$out" | grep -cE '^\s*\[ \]')
    if [ "$rc" != 0 ]; then caught=1; fi
    echo "  $t: exit=$rc rows_failed=$n"
    printf '%s\n' "$out" | grep -E '^\s*\[ \]' | sed 's/^/    /'
  done
  if [ "$caught" = 0 ]; then
    echo "  !! MUTANT $m WAS NOT CAUGHT — the tests are vacuous for this defect"
    BAD=1
  else
    echo "  mutant $m correctly REJECTED"
  fi
done

echo
if [ "$BAD" = 0 ]; then
  echo "NEGATIVE CONTROL PASSED: every mutant was rejected"
else
  echo "NEGATIVE CONTROL FAILED"
fi
exit $BAD
