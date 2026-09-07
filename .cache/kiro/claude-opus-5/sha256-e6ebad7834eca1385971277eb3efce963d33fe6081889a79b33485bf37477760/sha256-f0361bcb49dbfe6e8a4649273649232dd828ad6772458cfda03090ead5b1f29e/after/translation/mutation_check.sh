#!/usr/bin/env bash
# Mutation test: prove the differential suite can actually DETECT a divergence.
# Each mutant is a deliberately-wrong copy of src/lib.rs, built into its own
# .so; the suite is then pointed at it via RUST_SO and MUST fail.
set -u
cd "$(dirname "$0")"
ROOT=$PWD
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

mutate() { # $1=name  $2=sed program
  rm -rf "$WORK/m"; mkdir -p "$WORK/m"
  cp -r "$ROOT/src" "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$WORK/m/"
  perl -0pi -e "$2" "$WORK/m/src/lib.rs"
  if cmp -s "$ROOT/src/lib.rs" "$WORK/m/src/lib.rs"; then
    echo "MUTANT $1: SED DID NOT APPLY"; return 2
  fi
  ( cd "$WORK/m" && cargo build --release >/dev/null 2>&1 ) || { echo "MUTANT $1: build failed"; return 2; }
  echo "$WORK/m/target/release/libdequantize_granule_lib.so"
}

check() { # $1=name  $2=so  $3..=test filters
  local name=$1 so=$2; shift 2
  local failed=0
  for t in "$@"; do
    if ! RUST_SO="$so" timeout 600 cargo test --release --test phase_b_configs --test phase_c_errors "$t" >/dev/null 2>&1; then
      failed=1; break
    fi
  done
  if [ $failed -eq 1 ]; then echo "MUTANT $name: DETECTED (suite failed)  [good]"
  else echo "MUTANT $name: *** NOT DETECTED -- the suite is blind here ***"; fi
}

run() { # $1=name $2=sed $3..=filters
  local name=$1 sed=$2; shift 2
  local so; so=$(mutate "$name" "$sed") || { echo "$so"; return; }
  check "$name" "$so" "$@"
}

run "guard-ge"      's/if \(\*bs\)\.pos > \(\*bs\)\.limit \{/if (*bs).pos >= (*bs).limit {/' e03_ c21_
run "choff-19"      's/choff = 18 - choff;/choff = 19 - choff;/g'                             c05_ c19_
run "shl-not-masked" 's/cache \|= next\.wrapping_shl\(shl as u32\);/cache |= if shl >= 32 { 0 } else { next << shl };/' c11_ e16_
run "sat-sub"       's/\(code % m\)\.wrapping_sub\(m \/ 2\) as c_int/(code % m).saturating_sub(m \/ 2) as c_int/'        e20_ c09_
run "ba-masked-idx" 's/\*bitalloc\.offset\(i as isize\)/*bitalloc.offset((i \& 63) as isize)/'                           e21_ c15_
run "half-off-one"  's/1i32\.wrapping_shl\(\(ba - 1\) as u32\)\.wrapping_sub\(1\)/1i32.wrapping_shl((ba - 1) as u32)/'   c03_ e01_
run "pos-not-advanced" 's/\(\*bs\)\.pos = \(\*bs\)\.pos\.wrapping_add\(n\);\n    if \(\*bs\)\.pos > \(\*bs\)\.limit \{\n        return 0;\n    \}/if (*bs).pos.wrapping_add(n) > (*bs).limit { return 0; }\n    (*bs).pos = (*bs).pos.wrapping_add(n);/' e01_ e02_
run "mod-shift-wrap" 's/2i32\.wrapping_shl\(\(ba - 17\) as u32\)/2i32.wrapping_shl(((ba - 17) as u32).min(30))/'         e17_ c13_
