#!/usr/bin/env bash
# Sanity-check that the differential suite actually detects divergence: inject a
# deliberate bug into the Rust, confirm the suite FAILS, then restore.
# Every mutation must fail; a mutation that passes means the suite has a blind spot.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$ROOT/translation/src"
BACKUP="$(mktemp -d)"
cp -r "$SRC" "$BACKUP/src"
restore() { rm -rf "$SRC"; cp -r "$BACKUP/src" "$SRC"; }
trap 'restore; rm -rf "$BACKUP"' EXIT

check() { # name, config-op, config-repeat, expect(fail|pass)
  local name="$1" op="$2" rep="$3"
  if timeout 600 "$ROOT/run_tests.sh" "$op" "$rep" >/dev/null 2>&1; then
    echo "BLIND SPOT: mutation '$name' PASSED under $op,$rep"
    return 1
  else
    echo "detected:   mutation '$name' correctly fails under $op,$rep"
    return 0
  fi
}

bad=0

# 1. INIT_mul seeded with 0 instead of 1 (mdmacros.h:58).
restore
sed -i 's|^pub const INIT: c_int = 1;|pub const INIT: c_int = 0;|' "$SRC/mdmacros.rs"
check "INIT_mul = 0" mul 3 || bad=1

# 2. DISPATCH_REP gains the `case 7` the C switch does not have (mdmacros.h:82-93).
restore
sed -i 's|^        _ => acc,|        7 => rep(acc, 7),\n        _ => acc,|' "$SRC/mdmacros.rs"
check "DISPATCH_REP has case 7" add 7 || bad=1

# 3. REPEAT off by one.
restore
sed -i 's|^pub const REPEAT: c_int = 5;|pub const REPEAT: c_int = 4;|' "$SRC/mdmacros.rs"
check "REPEAT 5 -> 4" add 5 || bad=1

# 4. STEP_mul drops the `+ 1` (mdmacros.h:50).
restore
sed -i 's|acc.wrapping_mul(i.wrapping_add(1))|acc.wrapping_mul(i)|' "$SRC/mdmacros.rs"
check "STEP_mul without +1" mul 4 || bad=1

# 5. op_sub with swapped operands.
restore
sed -i 's|    a.wrapping_sub(b)|    b.wrapping_sub(a)|' "$SRC/mdcore.rs"
check "op_sub operands swapped" sub 2 || bad=1

# 6. G_OP_NAME reports the wrong operation.
restore
sed -i 's|b"sub\\0"|b"SUB\\0"|' "$SRC/mdmacros.rs"
check "G_OP_NAME wrong" sub 5 || bad=1

# 7. printf text changed (catches format-string drift, not just return values).
restore
sed -i 's|helper.acc=|helper.ACC=|' "$SRC/mdcore.rs"
check "helper_call printf text" add 5 || bad=1

# 8. atoi negative-overflow clamp regressed to -LONG_MAX (the real bug found).
restore
sed -i 's|            i64::MIN|            i64::MIN + 1|' "$SRC/cstdlib.rs"
check "atoi negative clamp off by one" add 5 || bad=1

# 9. helper_call returns r - acc instead of r + acc.
restore
sed -i 's|    r.wrapping_add(acc)|    r.wrapping_sub(acc)|' "$SRC/mdcore.rs"
check "helper_call sign flip" add 5 || bad=1

# 10. op_mul panics on overflow instead of wrapping (release mode: silently wrong
#     is impossible, but debug-style checked arithmetic would abort).
restore
sed -i 's|    a.wrapping_mul(b)|    a.checked_mul(b).unwrap_or(0)|' "$SRC/mdcore.rs"
check "op_mul clamps on overflow" mul 5 || bad=1

# 11. G_OP_NAME declared as an immutable `static`, which places it in
#     .data.rel.ro (read-only under RELRO) instead of C's writable .data.
restore
python3 - "$SRC/mdcore.rs" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace(
    'pub static mut G_OP_NAME: *const c_char = '
    '&OP_NAME_STORAGE as *const [u8; 4] as *const c_char;',
    'pub static G_OP_NAME: CStrPtr = '
    'CStrPtr(&OP_NAME_STORAGE as *const [u8; 4] as *const c_char);')
s = s.replace(
    'static OP_NAME_STORAGE: [u8; 4] = *OP_NAME_C;',
    'static OP_NAME_STORAGE: [u8; 4] = *OP_NAME_C;\n'
    '#[repr(transparent)]\npub struct CStrPtr(pub *const c_char);\n'
    'unsafe impl Sync for CStrPtr {}')
open(p, 'w').write(s)
PY
check "G_OP_NAME immutable (lands in .data.rel.ro)" add 5 || bad=1

# 12. INIT for add/sub seeded with 1 instead of 0.
restore
sed -i 's|^pub const INIT: c_int = 0;|pub const INIT: c_int = 1;|' "$SRC/mdmacros.rs"
check "INIT_add = 1" add 5 || bad=1

restore
if [[ $bad -eq 0 ]]; then
  echo "MUTATION CHECK: all 12 injected bugs were detected"
else
  echo "MUTATION CHECK: BLIND SPOTS FOUND"
fi
exit "$bad"
