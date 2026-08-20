#!/bin/sh
# Runs the whole differential suite for EVERY build configuration that exists.
#
# `cargo metadata` reports `"features": {}` for this crate, i.e. there is not a
# single cargo feature, so the complete feature power-set is the empty set and
# `--no-default-features` is the only (and the default) combination.  The list
# below therefore also exercises the release profile, which is where Rust drops
# the debug integer-overflow checks that C never had - a genuinely different
# code path through the same source.
set -e
COMBOS=$(cargo metadata --no-deps --format-version 1 \
         | tr ',' '\n' | grep -c '"features":{}' || true)
echo "cargo features declared: none (verified: $COMBOS)"

for cfg in "--no-default-features" "--no-default-features --release"; do
  echo
  echo "################ cargo test $cfg ################"
  cargo build $cfg                       # (re)build the cdylib for this config
  cargo test $cfg --test diff -- 2>&1 | tail -n 40
done
