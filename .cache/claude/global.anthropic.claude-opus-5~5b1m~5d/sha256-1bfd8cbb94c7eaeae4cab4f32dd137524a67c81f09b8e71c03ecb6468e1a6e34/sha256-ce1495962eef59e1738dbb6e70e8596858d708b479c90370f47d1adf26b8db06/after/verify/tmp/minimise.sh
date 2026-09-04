#!/bin/bash
cd $HARVEST_WORKDIR
W=${2:-5}
differs() { local h="$1"
  a=$(./verify/tmp/mini_c "$h" $W); b=$(./verify/tmp/mini_rs "$h" $W)
  [ "$a" != "$b" ]
}
H="$1"
# 1) shrink from the end
while : ; do
  L=${#H}; [ $L -le 2 ] && break
  T=${H:0:$((L-2))}
  if differs "$T"; then H="$T"; else break; fi
done
# 2) try zeroing each byte, left to right
n=$(( ${#H} / 2 ))
for ((i=0;i<n;i++)); do
  pre=${H:0:$((i*2))}; post=${H:$((i*2+2))}
  for cand in 00 01; do
    T="$pre$cand$post"
    if [ "$T" != "$H" ] && differs "$T"; then H="$T"; break; fi
  done
done
echo "MINIMAL: $H  (${#H} hex chars = $(( ${#H}/2 )) bytes)"
./verify/tmp/mini_c "$H" $W
./verify/tmp/mini_rs "$H" $W
