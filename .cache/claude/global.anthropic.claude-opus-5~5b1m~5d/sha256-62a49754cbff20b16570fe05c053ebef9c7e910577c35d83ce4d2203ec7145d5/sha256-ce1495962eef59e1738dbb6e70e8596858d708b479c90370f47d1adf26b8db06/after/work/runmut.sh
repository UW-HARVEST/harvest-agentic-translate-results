#!/bin/bash
R=$HARVEST_WORKDIR
cd $R/translation
while IFS='|' read -r label expr; do
  [ -z "$label" ] && continue
  case "$expr" in *"XX"*) echo "SKIP (multiline) $label"; continue;; esac
  cp $R/work/lib.rs.bak src/lib.rs
  perl -0pi -e "$expr" src/lib.rs 2>/dev/null || { echo "PERL-ERR $label"; continue; }
  if diff -q src/lib.rs $R/work/lib.rs.bak >/dev/null; then
    echo "NOOP     $label"
    continue
  fi
  if ! cargo build --offline -q 2>/dev/null || ! cargo build --release --offline -q 2>/dev/null; then
    echo "BUILDERR $label"
    continue
  fi
  out=$(cargo test --offline -q 2>&1 || true)
  names=$(echo "$out" | grep -oE "^[a-z0-9_]+ --- FAILED" | sed 's/ --- FAILED//' | sort -u)
  n=$(echo "$names" | grep -c .)
  if [ "$n" -eq 0 ]; then
    echo "SURVIVED $label"
  else
    echo "KILLED($n) $label   [$(echo $names | cut -c1-120)]"
  fi
done < $R/work/mutations.txt
cp $R/work/lib.rs.bak src/lib.rs
cargo build --offline -q 2>/dev/null; cargo build --release --offline -q 2>/dev/null
