#!/bin/bash
# Build every differential harness twice (against the reference C libzstd.so and
# against the Rust libzstd.so) and diff the traces.
set -u
W=$HARVEST_WORKDIR
cd "$W"
CLIB=$W/verify/cbuild
RLIB=$W/verify/rustlib
INC="-Ic_src/src/include -Ic_src/src -Ic_src/src/common -Ic_src/src/legacy -Ic_src/src/deprecated -Ic_src/src/dictBuilder"
cp translation/target/release/libzstd.so "$RLIB/" || exit 1

build() { # name
  gcc -w -O1 -o verify/$1_c  verify/$1.c $INC -L"$CLIB" -l:libzstd.so -Wl,-rpath,"$CLIB" || return 1
  gcc -w -O1 -o verify/$1_rs verify/$1.c $INC -L"$RLIB" -l:libzstd.so -Wl,-rpath,"$RLIB" || return 1
}
one() { # name [args...]
  local n=$1; shift
  local tag="${*:-all}"; tag=${tag// /_}
  timeout 900 ./verify/${n}_c  "$@" > verify/r_${n}_${tag}_c.txt  2>/dev/null; local ec=$?
  timeout 900 ./verify/${n}_rs "$@" > verify/r_${n}_${tag}_rs.txt 2>/dev/null; local er=$?
  local lines; lines=$(wc -l < verify/r_${n}_${tag}_c.txt)
  if diff -q verify/r_${n}_${tag}_c.txt verify/r_${n}_${tag}_rs.txt >/dev/null; then
     printf "  %-28s exit %-4s/%-4s lines=%-8s IDENTICAL\n" "$n $tag" "$ec" "$er" "$lines"
  else
     printf "  %-28s exit %-4s/%-4s lines=%-8s *** DIFFERS(%s) ***\n" "$n $tag" "$ec" "$er" "$lines" \
        "$(diff verify/r_${n}_${tag}_c.txt verify/r_${n}_${tag}_rs.txt | grep -c '^[<>]')"
  fi
}

echo "== symbol table =="
./verify/diff_syms.sh | head -5

for h in harness big adv entropy legacy disp stress; do build $h || echo "  BUILD FAILED: $h"; done

echo "== harness (core public API) =="        ; one harness
echo "== big (large/corrupt/strategies) =="   ; one big
echo "== adv (advanced+experimental API) ==" ; for p in $(./verify/adv_c --list); do one adv $p; done
echo "== entropy (FSE/HUF/HIST/XXH/sort) ==" ; for s in $(./verify/entropy_c --list); do one entropy $s; done
echo "== legacy (v0.1 .. v0.7 + ZBUFFv0x) ==" ; for p in P0 P1 P2 P3 P4 P5 P6 P7 P8 P9; do one legacy $p; done
echo "== dictBuilder stderr diagnostics =="  ; one disp
./verify/disp_c  > /dev/null 2> verify/r_disp_err_c.txt
./verify/disp_rs > /dev/null 2> verify/r_disp_err_rs.txt
if diff -q verify/r_disp_err_c.txt verify/r_disp_err_rs.txt >/dev/null; then
  printf "  %-28s lines=%-8s IDENTICAL\n" "disp stderr" "$(wc -l < verify/r_disp_err_c.txt)"
else
  printf "  %-28s *** DIFFERS ***\n" "disp stderr"
fi
echo "== stress (>3.5GB, overflow correction) ==" ; one stress 3800
