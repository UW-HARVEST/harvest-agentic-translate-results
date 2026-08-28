#!/bin/bash
R=$HARVEST_WORKDIR
cd $R/translation
cp $R/work/lib.rs.bak src/lib.rs
sed -i "$1" src/lib.rs
if diff -q src/lib.rs $R/work/lib.rs.bak >/dev/null; then
  echo "MUTATION-NOOP (sed matched nothing): $2"
  cp $R/work/lib.rs.bak src/lib.rs
  exit 9
fi
if ! cargo build --offline -q 2>/dev/null || ! cargo build --release --offline -q 2>/dev/null; then
  echo "MUTATION-BUILD-FAIL: $2"
  cp $R/work/lib.rs.bak src/lib.rs
  exit 8
fi
out=$(cargo test --offline -q 2>&1 || true)
names=$(echo "$out" | grep -oE "^[a-z0-9_]+ --- FAILED" | sed 's/ --- FAILED//' | sort -u)
n=$(echo "$names" | grep -c . )
echo "MUTATION [$2]: $n failing tests: $(echo $names | head -c 300)"
cp $R/work/lib.rs.bak src/lib.rs
cargo build --offline -q 2>/dev/null; cargo build --release --offline -q 2>/dev/null
