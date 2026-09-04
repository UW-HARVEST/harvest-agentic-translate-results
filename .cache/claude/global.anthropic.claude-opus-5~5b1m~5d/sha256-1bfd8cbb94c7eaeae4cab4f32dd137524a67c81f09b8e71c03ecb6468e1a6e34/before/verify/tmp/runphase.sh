#!/bin/bash
p="$1"
cd $HARVEST_WORKDIR
t0=$SECONDS; ./verify/legacy_c  $p > verify/tmp/c_$p.txt 2>verify/tmp/c_$p.err; rc_c=$?; t1=$SECONDS
./verify/legacy_rs $p > verify/tmp/r_$p.txt 2>verify/tmp/r_$p.err; rc_r=$?; t2=$SECONDS
echo "$p: C rc=$rc_c ${((t1-t0))}s  RS rc=$rc_r"
