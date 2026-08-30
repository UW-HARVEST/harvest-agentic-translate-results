#!/usr/bin/env bash
# End-to-end check: run the C KAT driver (app/src/PQCgenKAT_sign.c) and the Rust
# driver (src/main.rs) for every configuration and compare their transcript
# digests.  This exercises randombytes_init / randombytes / crypto_sign_keypair
# / crypto_sign / crypto_sign_open over 7 message sizes in one shot.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
THASHES="${THASHES:-robust simple}"

fail=0
for b in $BACKENDS; do
  for s in $SECPARS; do
    for t in $THASHES; do
      tag="$b-$s-$t"
      cout=$(cd "$ROOT/c_src/build-$tag/app" && timeout 600 ./driver 2>&1 | tail -1)
      rout=$(cd "$CRATE" && CARGO_TARGET_DIR="target/$tag" timeout 600 cargo run \
              --offline --release --no-default-features --features "$b,$t,$s" \
              --bin driver 2>/dev/null | tail -1)
      if [ "$cout" = "$rout" ] && [ -n "$cout" ]; then
        echo "MATCH $tag  $cout"
      else
        echo "DIFF  $tag"
        echo "   C   : $cout"
        echo "   Rust: $rout"
        fail=1
      fi
    done
  done
done
exit $fail
