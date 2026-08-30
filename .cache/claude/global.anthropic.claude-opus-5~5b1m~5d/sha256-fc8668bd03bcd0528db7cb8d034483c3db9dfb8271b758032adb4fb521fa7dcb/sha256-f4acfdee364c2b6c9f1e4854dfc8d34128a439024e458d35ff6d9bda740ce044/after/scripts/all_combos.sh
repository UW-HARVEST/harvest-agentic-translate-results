#!/bin/bash
# Enumerate every valid feature combination (backend x thash x secpar = 48).
BACKENDS="haraka sha2 shake blake"
THASHES="robust simple"
SECPARS="128s 128f 192s 192f 256s 256f"
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      echo "$b,$t,$s"
    done
  done
done
