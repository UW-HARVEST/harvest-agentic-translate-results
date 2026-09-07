#!/bin/bash
# Enumerate every valid feature combination (mirrors the CMake cache variables).
for b in haraka sha2 shake blake shake256; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      echo "$b,$t,$s"
    done
  done
done
