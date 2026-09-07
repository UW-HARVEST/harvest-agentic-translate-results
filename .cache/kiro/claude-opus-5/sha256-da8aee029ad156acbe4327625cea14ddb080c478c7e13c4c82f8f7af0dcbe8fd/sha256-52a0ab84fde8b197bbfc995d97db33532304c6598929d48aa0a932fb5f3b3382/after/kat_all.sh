#!/bin/bash
# End-to-end differential check: run the C `driver` (PQCgenKAT_sign.c) and the
# Rust `driver` (src/main.rs) for every feature combination and compare the KAT
# transcript digests they print.
ROOT="$(cd "$(dirname "$0")" && pwd)"
FAIL=0
for be in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      combo="$be,$th,$sp"
      cdrv="$ROOT/cbuild/${be}_${th}_${sp}/app"
      cout=$(cd "$cdrv" && LD_LIBRARY_PATH=".:../lib/$be" timeout 600 ./driver 2>/dev/null | tail -1)
      (cd "$ROOT/translation" && timeout 600 cargo build --release --no-default-features \
          --features "$combo" --bin driver > /dev/null 2>&1) || { echo "BUILDFAIL $combo"; FAIL=1; continue; }
      rout=$(timeout 600 "$ROOT/translation/target/release/driver" 2>/dev/null | tail -1)
      if [ -n "$cout" ] && [ "$cout" == "$rout" ]; then
        echo "MATCH $combo  ${cout##*= }"
      else
        echo "DIFF  $combo"
        echo "   C:    $cout"
        echo "   Rust: $rout"
        FAIL=1
      fi
    done
  done
done
exit $FAIL
