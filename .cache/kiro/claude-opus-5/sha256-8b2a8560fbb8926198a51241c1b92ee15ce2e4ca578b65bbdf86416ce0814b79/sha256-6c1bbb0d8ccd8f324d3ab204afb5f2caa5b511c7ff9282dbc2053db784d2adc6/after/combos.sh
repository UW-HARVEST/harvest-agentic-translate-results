#!/bin/bash
# Enumerate every valid feature combination: backend x thash x secpar (x urandom)
for backend in haraka sha2 shake blake; do
  for thash in robust simple; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      echo "$backend,$thash,$secpar"
    done
  done
done
