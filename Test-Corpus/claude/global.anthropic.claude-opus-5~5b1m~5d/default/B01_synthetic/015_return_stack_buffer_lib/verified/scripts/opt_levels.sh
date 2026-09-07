#!/usr/bin/env bash
# Robustness check for the one place the C is UB (CWE-562 in helperBad):
# rebuild the C at every optimisation level and confirm the observable output of
# bad()/good()/driver(0)/driver(1) is the same as the Rust build's, so the
# translation does not depend on one particular compiler setting.
set -u
cd "$(dirname "$0")/.."
S=target/optprobe
mkdir -p "$S"
cat > "$S/probe.c" <<'C'
#include <dlfcn.h>
#include <stdio.h>
int main(int c, char **v) {
    setbuf(stdout, 0);
    void *h = dlopen(v[1], RTLD_NOW);
    if (!h) { printf("DLOPEN-FAIL %s\n", dlerror()); return 1; }
    void (*bad)(void)  = dlsym(h, "bad");
    void (*good)(void) = dlsym(h, "good");
    void (*drv)(int)   = dlsym(h, "driver");
    printf("[bad]"); bad();
    printf("[good]"); good();
    printf("[d0]"); drv(0);
    printf("[d1]"); drv(1);
    printf("[end]");
    return 0;
}
C
gcc -O0 -o "$S/probe" "$S/probe.c" -ldl || exit 1

ref=""
rc=0
emit() { # $1=label $2=so
  out=$("$S/probe" "$2" | tr -d '\n')
  printf '%-22s %s\n' "$1" "$out"
  if [ -z "$ref" ]; then ref="$out"
  elif [ "$out" != "$ref" ]; then echo "  ^^ DIVERGES from reference"; rc=1; fi
}

for opt in -O0 -O1 -O2 -O3 -Os; do
  gcc "$opt" -fPIC -shared -I ../c_src/include -o "$S/d$opt.so" ../c_src/src/driver.c 2>/dev/null
  emit "C gcc $opt" "$S/d$opt.so"
done
emit "C cmake (reference)" ../c_src/build/libdriver.so
[ -f target/debug/libdriver.so ]   && emit "Rust debug"   target/debug/libdriver.so
[ -f target/release/libdriver.so ] && emit "Rust release" target/release/libdriver.so

rm -rf "$S"
[ $rc -eq 0 ] && echo "OK: identical observable output across all builds" \
              || echo "FAIL: observable output differs"
exit $rc
