#!/usr/bin/env bash
# Phase D automation: symbol diff + all feature combinations.
set -u
CSO=../c_src/build/libdriver.so
RSO=target/release/libdriver.so

echo "=== symbol diff (C defined -> missing from Rust) ==="
diff <(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$RSO" | awk '{print $NF}' | sort) > /dev/null \
  && echo "identical defined-symbol sets"
comm -23 <(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RSO" | awk '{print $NF}' | sort) \
  | sed 's/^/MISSING: /'
echo "missing count: $(comm -23 <(nm -D --defined-only "$CSO"|awk '{print $NF}'|sort) <(nm -D --defined-only "$RSO"|awk '{print $NF}'|sort) | wc -l)"

echo
echo "=== features declared in Cargo.toml ==="
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)
if [ -z "$FEATS" ]; then echo "(none - single default configuration only)"; fi

echo
for combo in "default" "--no-default-features" "--all-features"; do
  echo "=== cargo test $combo ==="
  if [ "$combo" = "default" ]; then FLAGS=""; else FLAGS="$combo"; fi
  timeout 600 cargo build --release --offline $FLAGS >/dev/null 2>&1 || { echo "BUILD FAILED"; continue; }
  timeout 600 cargo test --offline $FLAGS -- --test-threads=1 2>&1 | grep -E "test result|FAILED"
done
