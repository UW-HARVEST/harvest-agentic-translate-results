#!/bin/bash
# Whole-program check: run the C `driver` (app/src/PQCgenKAT_sign.c) and the
# Rust `driver` binary for every feature combination and compare the KAT
# transcript digest they print.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
pass=0; fail=0
for backend in haraka sha2 shake blake; do
  for thash in robust simple; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      combo="${backend}/${thash}/${secpar}"
      dir="$ROOT/cbuild/${backend}_${secpar}_${thash}"
      "$ROOT/build_c.sh" "$backend" "$secpar" "$thash" >/dev/null || {
        echo "FAIL $combo (C build)"; fail=$((fail+1)); continue; }
      cout=$(cd "$dir/app" && timeout 300 ./driver 2>&1)
      rout=$(cd "$ROOT/translation" && timeout 600 cargo run --quiet --release \
               --no-default-features --features "${backend},${thash},${secpar}" \
               --bin driver 2>&1)
      if [ "$cout" = "$rout" ] && [ -n "$cout" ]; then
        echo "ok   $combo  $cout"
        pass=$((pass+1))
      else
        echo "FAIL $combo"
        echo "      C   : $cout"
        echo "      Rust: $rout"
        fail=$((fail+1))
      fi
    done
  done
done
echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
