#!/bin/bash
cd $HARVEST_WORKDIR
W=${2:-5}
differs() { local h="$1"
  a=$(./verify/tmp/mini_c "$h" $W | head -1); b=$(./verify/tmp/mini_rs "$h" $W | head -1)
  # require BOTH to be non-error successes, and different
  case "$a" in *rv=E*) return 1;; esac
  case "$b" in *rv=E*) return 1;; esac
  [ "$a" != "$b" ]
}
H="$1"
differs "$H" || { echo "seed does not satisfy predicate"; exit 1; }
while : ; do
  L=${#H}; [ $L -le 8 ] && break
  T=${H:0:$((L-2))}
  if differs "$T"; then H="$T"; else break; fi
done
n=$(( ${#H} / 2 ))
for ((i=0;i<n;i++)); do
  pre=${H:0:$((i*2))}; post=${H:$((i*2+2))}
  for cand in 00 01 ff; do
    T="$pre$cand$post"
    if [ "$T" != "$H" ] && differs "$T"; then H="$T"; break; fi
  done
done
echo "MINIMAL(both-succeed): $H  ($(( ${#H}/2 )) bytes)"
./verify/tmp/mini_c "$H" $W
./verify/tmp/mini_rs "$H" $W
