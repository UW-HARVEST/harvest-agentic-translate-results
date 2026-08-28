#!/usr/bin/env bash
# Test-power check: deliberately break the Rust translation in ways that mimic
# realistic mis-translations, point the harness at the broken .so via RUST_SO,
# and assert that the suite FAILS. A differential suite that cannot fail proves
# nothing, so every mutant below must be caught by the named tests.
#
# Nothing in src/ or c_src/ is modified: each mutant is built from a copy.
set -uo pipefail
cd "$(dirname "$0")"

work="target/mutants"
rm -rf "${work}"; mkdir -p "${work}/src"
cat > "${work}/Cargo.toml" <<'EOF'
[package]
name = "mutant"
version = "0.1.0"
edition = "2021"
[lib]
name = "collided_lib"
path = "src/lib.rs"
crate-type = ["cdylib"]
[profile.release]
opt-level = 3
[workspace]
EOF

# Mutations are applied as LITERAL string replacements by mutate.py.
# format: name : tests that must fail
mutants=(
  "touching_boundary_cc:row14"
  "touching_boundary_ca:row20"
  "nan_suppressing_max:row03"
  "nan_suppressing_min:row05"
  "dot_operand_order:row12"
  "dot_sum_order:row12"
  "no_argument_swap:row29"
  "lenient_enum:row01 row04"
  "strict_separating_axis:row24 row25"
  "sub_operand_order:row09"
  "checked_null_read:row07"
  "quiet_nan_in_c2V:row01"
)

cat > "${work}/mutate.py" <<'PYEOF'
import sys
name, src, dst = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(src).read()
M = {
  # exact-touch boundary: `<` becomes `<=`
  # exact-touch boundary in c2CircletoCircle: `<` becomes `<=`
  "touching_boundary_cc": [("    (d2 < r2) as c_int\n}\n\n/// ```c\n/// int c2CircletoAABB",
                            "    (d2 <= r2) as c_int\n}\n\n/// ```c\n/// int c2CircletoAABB")],
  # same, in c2CircletoAABB
  "touching_boundary_ca": [("    (d2 < r2) as c_int\n}\n\n/// ```c\n/// int c2AABBtoAABB",
                            "    (d2 <= r2) as c_int\n}\n\n/// ```c\n/// int c2AABBtoAABB")],
  # NaN-suppressing std helpers instead of C's ternary
  "nan_suppressing_max": [("        if a.x > b.x { a.x } else { b.x },\n        if a.y > b.y { a.y } else { b.y },",
                           "        a.x.max(b.x),\n        a.y.max(b.y),")],
  "nan_suppressing_min": [("        if a.x < b.x { a.x } else { b.x },\n        if a.y < b.y { a.y } else { b.y },",
                           "        a.x.min(b.x),\n        a.y.min(b.y),")],
  # wrong mulss/addss operand order (wrong NaN payload propagation)
  "dot_operand_order": [("let y_product = mulss(b.y, a.y);", "let y_product = mulss(a.y, b.y);")],
  "dot_sum_order":     [("addss(y_product, x_product)", "addss(x_product, y_product)")],
  # forget that the C swaps the operands for AABB-vs-CIRCLE
  "no_argument_swap": [("            C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { read_circle(B) }, unsafe {\n                read_aabb(A)",
                        "            C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { read_circle(B) }, unsafe {\n                read_aabb(B)")],
  # accept out-of-range enum values instead of returning 0
  "lenient_enum": [("        _ => 0,\n    }\n}", "        _ => c2AABBtoAABB(unsafe { read_aabb(A) }, unsafe { read_aabb(B) }),\n    }\n}")],
  # separating-axis test with the wrong strictness
  "strict_separating_axis": [("let d0 = (B.max.x < A.min.x) as c_int;", "let d0 = (B.max.x <= A.min.x) as c_int;")],
  # reversed subtraction
  "sub_operand_order": [("    a.x -= b.x;", "    a.x = b.x - a.x;")],
  # use the checked (panicking) read instead of the raw load
  "checked_null_read": [("    let out: u32;\n    unsafe {\n        core::arch::asm!(",
                         "    return unsafe { (p as *const u32).read_unaligned() };\n    #[allow(unreachable_code)] let out: u32;\n    #[allow(unreachable_code)] unsafe {\n        core::arch::asm!(")],
  # quiet a NaN where the C merely copies it
  "quiet_nan_in_c2V": [("    a.x = x;\n    a.y = y;", "    a.x = if x.is_nan() { f32::from_bits(x.to_bits() | 0x0040_0000) } else { x };\n    a.y = y;")],
}
reps = M[name]
for old, new in reps:
    if old not in s:
        print(f"PATTERN-MISS {name}", file=sys.stderr); sys.exit(2)
    s = s.replace(old, new, 1)
open(dst, "w").write(s)
PYEOF

status=0
for m in "${mutants[@]}"; do
  name=${m%%:*}; want=${m#*:}
  if ! python3 "${work}/mutate.py" "${name}" src/lib.rs "${work}/src/lib.rs"; then
    echo "SKIP ${name}: pattern no longer present in src/lib.rs (update mutate.py)"
    status=1; continue
  fi
  compiled=0
  for prof in "" "--release"; do
    (cd "${work}" && timeout 600 cargo build ${prof} --target-dir build >/dev/null 2>&1) && compiled=1
  done
  if [ ${compiled} -eq 0 ]; then
    echo "SKIP ${name}: mutant did not compile"
    status=1; continue
  fi
  caught_all=1
  for t in ${want}; do
    caught_here=""
    for prof in "" "--release"; do
      pdir=${prof:+release}; pdir=${pdir:-debug}
      so="${PWD}/${work}/build/${pdir}/libcollided_lib.so"
      [ -f "${so}" ] || continue
      out=$(RUST_SO="${so}" timeout 600 cargo test ${prof} --test phase_b_valid --test phase_c_errors "${t}" 2>&1)
      if echo "${out}" | grep -qE "^test result: FAILED"; then
        caught_here="${caught_here}${caught_here:+,}${pdir}"
      fi
    done
    if [ -z "${caught_here}" ]; then
      echo "NOT CAUGHT: mutant '${name}' survives test '${t}' in either profile"
      caught_all=0; status=1
    else
      profiles_hit="${caught_here}"
    fi
  done
  [ ${caught_all} -eq 1 ] && echo "caught: ${name}  (by: ${want} — fails in: ${profiles_hit})"
done

rm -rf "${work}"
echo
if [ ${status} -eq 0 ]; then echo "ALL MUTANTS CAUGHT — the suite has real discriminating power"; else echo "SOME MUTANTS SURVIVED"; fi
exit ${status}
