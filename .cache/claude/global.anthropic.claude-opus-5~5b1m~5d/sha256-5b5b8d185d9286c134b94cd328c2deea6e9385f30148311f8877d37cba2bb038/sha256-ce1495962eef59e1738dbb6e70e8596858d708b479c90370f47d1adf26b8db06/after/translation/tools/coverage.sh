#!/usr/bin/env bash
# TRUE runtime symbol coverage: run every differential test with the harness's
# coverage recorder enabled, then diff the exercised set against `nm -D` on the
# C .so.
#
# t99_symbols is EXCLUDED on purpose: it dlsym's all 615 exports by design, so
# including it would make every symbol look "exercised" and the measurement
# meaningless.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/tmp/coverage.txt"
mkdir -p "$ROOT/tmp"
: > "$OUT"

cd "$ROOT/translation"
cargo build --release --offline >/dev/null 2>&1
cargo test  --release --offline --no-run >/dev/null 2>&1

for t in $(ls tests/*.rs | xargs -n1 basename | sed 's/\.rs$//' | grep -v '^t99_symbols$'); do
  printf '%-20s ' "$t"
  res=$(ZSTD_DIFF_COVERAGE="$OUT" timeout 600 cargo test --release --offline --test "$t" 2>&1 \
        | grep -oE 'test result: [a-z]+\. [0-9]+ passed; [0-9]+ failed' | head -1)
  echo "${res:-NO RESULT / CRASH}"
done

sort -u "$OUT" -o "$OUT"
nm -D --defined-only "$ROOT/c_src/build/libzstd.so" | awk '{print $3}' | sort -u > "$ROOT/tmp/c_syms.txt"
echo
echo "=== runtime symbol coverage ==="
echo "exercised: $(comm -12 "$ROOT/tmp/c_syms.txt" "$OUT" | wc -l) / $(wc -l < "$ROOT/tmp/c_syms.txt")"
comm -23 "$ROOT/tmp/c_syms.txt" "$OUT" > "$ROOT/tmp/uncovered.txt"
echo "not exercised: $(wc -l < "$ROOT/tmp/uncovered.txt")  (listed in tmp/uncovered.txt)"
