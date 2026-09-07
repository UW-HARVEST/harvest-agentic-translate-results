#!/bin/bash
# Dump + diff C vs Rust exported symbol sets for every backend.
ROOT="$(cd "$(dirname "$0")" && pwd)"
D="$ROOT/symdumps"; mkdir -p "$D"
# C: union over all secpar/thash for each backend
for b in blake haraka sha2 shake; do
  : > "$D/c-$b.raw"
  for d in "$ROOT"/cbuild/$b-*/; do
    while IFS= read -r f; do
      nm -D --defined-only "$f" | grep -v ' [wWvV] ' | awk '{print $3}' >> "$D/c-$b.raw"
    done < <(find "$d" -name '*.so')
  done
  sort -u "$D/c-$b.raw" > "$D/c-$b.txt"; rm -f "$D/c-$b.raw"
done
# Rust: one build per backend (128f/simple; symbol set is backend-determined)
cd "$ROOT/translation" || exit 1
for b in blake haraka sha2 shake; do
  cargo build --release --no-default-features --features "$b,simple,128f" >/dev/null 2>&1 \
    || { echo "RUST BUILD FAIL $b"; continue; }
  nm -D --defined-only target/release/libsphincs_core_det.so | grep -v ' [wWvV] ' \
    | awk '{print $3}' | grep -v '^_' | sort -u > "$D/r-$b.txt"
done
for b in blake haraka sha2 shake; do
  m=$(comm -23 "$D/c-$b.txt" "$D/r-$b.txt" | tr '\n' ' ')
  e=$(comm -13 "$D/c-$b.txt" "$D/r-$b.txt" | tr '\n' ' ')
  echo "[$b] MISSING_IN_RUST: ${m:-<none>}"
  echo "[$b] EXTRA_IN_RUST:   ${e:-<none>}"
done
