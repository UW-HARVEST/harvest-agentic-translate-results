#!/usr/bin/env bash
# Emit every valid feature combination (one per line) for the driver crate.
#
# Axes mirror c_src/CMakeLists.txt:
#   OP     in {add, sub, mul}                    (default add, via #ifndef OP)
#   REPEAT in {0,1,2,3,4,5,6,7}                  (default 5, via #ifndef REPEAT)
#
# Each axis has three spellings in Cargo.toml: the bare CMake value
# ("add"/"5"), a readable alias ("op_add"/"repeat_5"), and -- for REPEAT --
# both. Omitting an axis entirely exercises the header's #ifndef fallback.
for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    echo "$op,repeat_$r"
  done
done
# Alias spellings.
for op in op_add op_sub op_mul; do
  for r in 0 1 2 3 4 5 6 7; do
    echo "$op,$r"
  done
done
# #ifndef fallbacks: missing OP (=> add), missing REPEAT (=> 5), both missing.
for r in 0 1 2 3 4 5 6 7; do echo "repeat_$r"; done
for op in add sub mul op_add op_sub op_mul; do echo "$op"; done
echo ""
