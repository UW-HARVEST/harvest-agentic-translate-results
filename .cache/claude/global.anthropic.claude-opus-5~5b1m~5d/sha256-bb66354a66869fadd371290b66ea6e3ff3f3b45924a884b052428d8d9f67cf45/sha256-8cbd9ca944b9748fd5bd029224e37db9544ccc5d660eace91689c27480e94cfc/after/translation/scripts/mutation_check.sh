#!/usr/bin/env bash
# Mutation harness: apply a one-line perturbation to translation/src/lib.rs,
# rebuild the cdylib properly, and confirm the differential suite CATCHES it.
# Any "*** MISSED ***" line is a hole in the test suite.
cd "$(dirname "$0")/.." || exit 1
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp src/lib.rs "$BAK"
trap 'cp "$BAK" src/lib.rs; rm -f "$BAK"' EXIT

run() {
  local name="$1"
  local expr="$2"
  cp "$BAK" src/lib.rs
  perl -0pi -e "$expr" src/lib.rs || { echo "PERL-FAIL: $name"; return; }
  if cmp -s "$BAK" src/lib.rs; then echo "MUTATION-NOT-APPLIED: $name"; return; fi
  touch src/lib.rs
  if ! cargo build --offline --release >/dev/null 2>&1; then
    echo "SKIP(build fail): $name"; return
  fi
  local out nfail
  out=$(timeout 600 cargo test --offline --release 2>&1)
  nfail=$(printf '%s\n' "$out" | grep -cE '^test .* FAILED$')
  if [ "$nfail" -gt 0 ]; then
    echo "CAUGHT  ($nfail failing tests): $name"
  else
    echo "*** MISSED ***: $name"
  fi
}

run "M1  tail d[3] sign-ext -> zero-ext"        's/data \|= \(\(\*d\.add\(3\) as i32\) << 24\) as usize;/data |= (*d.add(3) as usize) << 24;/'
run "M2  body lo sign-ext -> zero-ext"          's/data = lo as usize;/data = (lo as u32) as usize;/'
run "M3  body hi sign-ext -> zero-ext"          's/data \|= \(\(hi as usize\) << 16\) << 16;/data |= (((hi as u32) as usize) << 16) << 16;/'
run "M4  tail arm5 shift 20+20 -> 19+20"        's/data \|= \(\(\*d\.add\(5\) as usize\) << 20\) << 20;/data |= ((*d.add(5) as usize) << 19) << 20;/'
run "M5  tail arm6 shift 24+24 -> 24+23"        's/data \|= \(\(\*d\.add\(6\) as usize\) << 24\) << 24;/data |= ((*d.add(6) as usize) << 24) << 23;/'
run "M6  len<<56 -> len<<48"                    's/data = len << \(SIZE_T_BITS - 8\);/data = len << (SIZE_T_BITS - 16);/'
run "M7  final rounds 4 -> 3"                   's/while j < 4 \{/while j < 3 {/'
run "M8  v2 ^= 0xff -> 0xfe"                    's/v2 \^= 0xff;/v2 ^= 0xfe;/'
run "M9  sipround rot 13 -> 14"                 's/rotate_left\(\*v1, 13\)/rotate_left(*v1, 14)/'
run "M10 sipround rot 21 -> 20"                 's/rotate_left\(\*v3, 21\)/rotate_left(*v3, 20)/'
run "M11 fall-through: arm4 guard narrowed"     's/if rem >= 4 && rem <= 7 \{/if rem == 4 {/'
run "M12 v0 seed ^seed -> ^!seed"               's/\.wrapping_add\(0x70736575\) \^ seed;/.wrapping_add(0x70736575) ^ !seed;/'
run "M13 siphash: z++ before store"             's/mem\[i as usize\] = z as u8;/{ z = z.wrapping_add(1); mem[i as usize] = z as u8; }/'
run "M14 siphash: printf %02x -> %2x"           's/0x%02x, /0x%2x, /'
run "M15 body-loop cond <= -> <"                's/while i \+ sz <= len \{/while i + sz < len {/'
run "M16 tail arm1 guard dropped"               's/if rem >= 1 && rem <= 7 \{/if rem >= 2 \&\& rem <= 7 {/'
run "M17 first-round count 2 -> 1 (tail)"       's/    v0 \^= data;\n    v2 \^= 0xff;/    v0 ^= data; v0 = v0.wrapping_add(1);\n    v2 ^= 0xff;/'
run "M18 siphash: hash>>(j*8) -> >>(j*4)"       's/\(hash >> \(j \* 8\)\)/(hash >> (j * 4))/'
run "M19 return v0^v1^v2^v3 -> v0^v1^v2"        's/    v0 \^ v1 \^ v2 \^ v3\n\}/    v0 ^ v1 ^ v2\n}/'
run "M20 body: skip v0 ^= data"                 's/        v0 \^= data;\n\n        i \+= sz;/\n        i += sz;/'

cp "$BAK" src/lib.rs
touch src/lib.rs
cargo build --offline --release >/dev/null 2>&1
cargo build --offline >/dev/null 2>&1
echo "=== reverted src/lib.rs to original and rebuilt ==="
cmp "$BAK" src/lib.rs && echo "revert verified: identical to backup"
