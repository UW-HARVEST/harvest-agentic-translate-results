#!/usr/bin/env bash
# Negative control for the differential suite.
#
# A passing test suite only means something if it can fail. This script builds
# deliberately-wrong copies of the Rust library in a scratch directory (the real
# `translation/src` is never touched) and asserts that the suite REJECTS each
# one. If a mutant survives, the suite has a blind spot.
#
# Usage:  translation/scripts/mutation-check.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
TIMEOUT=${TIMEOUT:-600}

cp -r "$CRATE/src" "$CRATE/build.rs" "$CRATE/Cargo.toml" "$CRATE/.cargo" "$WORK/"

# name|kind|python-expression-over-`s`
#   kind=lib     -> patch src/lib.rs
#   kind=build   -> patch build.rs
#   kind=cargocfg-> patch .cargo/config.toml
MUTANTS=(
# Drop the NULL guard in printLine (the library's only rejection branch).
'nonullcheck|lib|s.replace("        \"cmp qword ptr [rbp - 8], 0\",   // if (line != NULL)\n        \"je 2f\",\n", "        \"cmp qword ptr [rbp - 8], 0\",   // if (line != NULL)\n")'
# Test only the low byte of driver's selector instead of the full 32 bits.
'lowbyte|lib|s.replace("\"cmp dword ptr [rbp - 4], 0\",", "\"cmp byte ptr [rbp - 4], 0\",")'
# Change driver frame size -> moves the depth bad/good run at.
'driver_frame32|lib|s.replace("\"sub rsp, 16\",\n        \"mov dword ptr [rbp - 4], edi\"", "\"sub rsp, 32\",\n        \"mov dword ptr [rbp - 4], edi\"")'
# Change printLine frame size -> changes what it clobbers below the caller.
'printline_frame32|lib|s.replace("\"sub rsp, 16\",\n        \"mov qword ptr [rbp - 8], rdi\"", "\"sub rsp, 32\",\n        \"mov qword ptr [rbp - 8], rdi\"")'
# bad() reads a different stack word than the C compiler chose.
'badslot|lib|s.replace("\"mov rax, qword ptr [rbp - 8]\", // uninitialized read", "\"mov rax, qword ptr [rbp - 16]\", // uninitialized read")'
# good() stores "string" into a different slot, so it no longer lands where a
# later bad() looks.
'goodslot|lib|s.replace("\"mov qword ptr [rbp - 8], rax\", // data = \"string\";", "\"mov qword ptr [rbp - 16], rax\", // data = \"string\";")'
# printLine emits line+1 instead of line (content off-by-one).
'offbyone|lib|s.replace("        \"mov rax, qword ptr [rbp - 8]\",\n        \"mov rdi, rax\",\n        \"call {puts}\",", "        \"mov rax, qword ptr [rbp - 8]\",\n        \"lea rdi, [rax + 1]\",\n        \"call {puts}\",")'
# Eager PLT binding: skips _dl_runtime_resolve, which the C runs on the first
# call from driver and which overwrites the word bad() then reads.
'nonlazy|build|s.replace("-Wl,-z,lazy", "-Wl,-z,now")'
)

pass=0
fail=0
for m in "${MUTANTS[@]}"; do
  name=${m%%|*}; rest=${m#*|}
  kind=${rest%%|*}; expr=${rest#*|}
  case "$kind" in
    lib)      target="src/lib.rs" ;;
    build)    target="build.rs" ;;
    cargocfg) target=".cargo/config.toml" ;;
  esac

  cp "$WORK/$target" "$WORK/$target.orig"
  TARGET="$target" EXPR="$expr" python3 - "$WORK" <<'PY'
import os, sys
work = sys.argv[1]
p = os.path.join(work, os.environ["TARGET"])
s = open(p).read()
out = eval(os.environ["EXPR"])
if out == s:
    sys.stderr.write("MUTATION DID NOT APPLY: %s\n" % os.environ["EXPR"][:80])
    sys.exit(1)
open(p, "w").write(out)
PY
  if [ $? -ne 0 ]; then
    echo "SKIP  $name (patch did not apply — source drifted)"
    fail=$((fail+1)); cp "$WORK/$target.orig" "$WORK/$target"; continue
  fi

  ( cd "$WORK" && timeout "$TIMEOUT" cargo build --release >/dev/null 2>&1 )
  if [ ! -f "$WORK/target/release/libdriver.so" ]; then
    echo "SKIP  $name (mutant did not build)"
    fail=$((fail+1)); cp "$WORK/$target.orig" "$WORK/$target"; continue
  fi
  cp "$WORK/target/release/libdriver.so" "$WORK/mutant-$name.so"
  cp "$WORK/$target.orig" "$WORK/$target"

  out=$( cd "$CRATE" && DRIVER_RUST_SO="$WORK/mutant-$name.so" \
         timeout "$TIMEOUT" cargo test --release --test differential -- --test-threads=1 2>&1 )
  if echo "$out" | grep -qE 'FAILED|SIGSEGV|SIGABRT'; then
    detected=$(echo "$out" | grep -cE '\.\.\. FAILED$')
    echo "OK    $name rejected ($detected test(s) failed$(echo "$out" | grep -q SIGSEGV && echo ', runner faulted'))"
    pass=$((pass+1))
  else
    echo "SURVIVED  $name  <-- BLIND SPOT: the suite accepted a wrong library"
    fail=$((fail+1))
  fi
done

echo
echo "mutants rejected: $pass    not rejected: $fail"
[ "$fail" -eq 0 ]
