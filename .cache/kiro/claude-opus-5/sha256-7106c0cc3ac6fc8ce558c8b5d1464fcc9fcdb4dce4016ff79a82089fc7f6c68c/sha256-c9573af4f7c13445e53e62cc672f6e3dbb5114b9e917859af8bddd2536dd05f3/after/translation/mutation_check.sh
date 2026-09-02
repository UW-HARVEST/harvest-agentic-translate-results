#!/usr/bin/env bash
# Mutation test: prove the differential suite actually detects divergence.
#
# Each mutation is a small, plausible mis-translation of the C. Every one MUST
# make at least one test fail, otherwise the suite has a blind spot.
# Mutations are applied with `perl -0777` so they can span lines.
# src/lib.rs is restored unconditionally on exit.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

# "name<TAB>perl -0777 -pe expression"
MUTATIONS=(
  $'fallcalc: flag test > becomes >=\ts/if param3 > OCTAL_FLAG/if param3 >= OCTAL_FLAG/'
  $'fallcalc: flag ORs 0100 instead of 0200\ts/result \\|= OCTAL_FLAG;/result |= OCTAL_MASK_2;/'
  $'fallcalc: final mask 0777 becomes 0377\ts/result &= OCTAL_MASK_1;\\n\\n    result\\n/result &= 0o377;\\n\\n    result\\n/'
  $'fallcalc: param3 % 5 becomes % 6\ts/param3.wrapping_rem\\(5\\)/param3.wrapping_rem(6)/'
  $'fallcalc: param4 % 10 becomes % 9\ts/param4.wrapping_rem\\(10\\)/param4.wrapping_rem(9)/'
  $'fallcalc: param4 %10+1 becomes +2\ts/wrapping_rem\\(10\\).wrapping_add\\(1\\)/wrapping_rem(10).wrapping_add(2)/'
  $'fallcalc: 3.7 coefficient becomes 3.75\ts/\\* 3.7 \\+/* 3.75 +/'
  $'fallcalc: 2.3 coefficient becomes 2.35\ts/\\* 2.3 -/* 2.35 -/'
  $'fallcalc: 0.5 coefficient becomes 0.55\ts/\\* 0.5;/* 0.55;/'
  $'fallcalc: floating_calc adds instead of subtracts\ts/\\) \\* 2.3 - \\(param3/) * 2.3 + (param3/'
  $'fallcalc: base_value uses 0200 not 0100\ts/param1.wrapping_mul\\(OCTAL_MASK_2\\)/param1.wrapping_mul(OCTAL_FLAG)/'
  $'fallcalc: array_size 5 becomes 4\ts/let array_size: c_int = 5;/let array_size: c_int = 4;/'
  $'fallcalc: nested multiplier 1.5 becomes 1.25\ts/, 1.5\\)/, 1.25)/'
  $'fallcalc: array fill (i+1)*8 becomes i*8\ts/i.wrapping_add\\(1\\).wrapping_mul\\(OCTAL_BASE\\)/i.wrapping_mul(OCTAL_BASE)/'
  $'fallcalc: last_element off by one\ts/data_array.offset\\(array_size as isize\\).offset\\(-1\\)/data_array.offset(array_size as isize).offset(-2)/'
  $'fallcalc: drops the reverse_sum term\ts/.wrapping_add\\(reverse_sum\\)/.wrapping_add(0)/'
  $'fallcalc: drops the converted term\ts/.wrapping_add\\(converted\\)/.wrapping_add(0)/'
  $'fallcalc: drops the alloc_result term\ts/.wrapping_add\\(alloc_result\\)/.wrapping_add(0)/'
  $'safe_double_to_int: NaN returns 1 not 0\ts/if d.is_nan\\(\\) \\{\\n        return 0;/if d.is_nan() {\\n        return 1;/'
  $'safe_double_to_int: inf signs swapped\ts/if d > 0.0 \\{ INT_MAX \\} else \\{ INT_MIN \\}/if d > 0.0 { INT_MIN } else { INT_MAX }/'
  $'safe_double_to_int: INT_MAX guard off by one\ts/if d >= INT_MAX as c_double/if d >= (INT_MAX - 1) as c_double/'
  $'safe_double_to_int: INT_MIN guard off by one\ts/if d <= INT_MIN as c_double/if d <= (INT_MIN + 1) as c_double/'
  $'safe_double_to_int: truncates toward -inf\ts/    d as c_int\\n\\}/    d.floor() as c_int\\n}/'
  $'safe_double_to_int: rounds instead of truncating\ts/    d as c_int\\n\\}/    d.round() as c_int\\n}/'
  $'switch arm 0: drops the *8\ts/result = result.wrapping_mul\\(OCTAL_BASE\\);/result = result;/'
  $'switch arm 1: drops the +0200\ts/            result = result.wrapping_add\\(OCTAL_FLAG\\);\\n            result &= OCTAL_MASK_1;\\n        \\}\\n        2 =>/            result \\&= OCTAL_MASK_1;\\n        }\\n        2 =>/'
  $'switch arm 2: drops the 0777 mask\ts/        2 => \\{\\n            result &= OCTAL_MASK_1;\\n        \\}/        2 => {}/'
  $'switch arm 2: masks with 0377\ts/        2 => \\{\\n            result &= OCTAL_MASK_1;\\n        \\}/        2 => {\\n            result \\&= 0o377;\\n        }/'
  $'switch arm 3: forgets the +0100 fallthrough\ts/result = result.wrapping_mul\\(3\\);/result = result.wrapping_mul(3); return result;/'
  $'switch arm 3: gains a 0777 mask\ts/result = result.wrapping_mul\\(3\\);/result = result.wrapping_mul(3) \\& OCTAL_MASK_1;/'
  $'switch arm 4: +0100 becomes +0200\ts/        4 => \\{\\n            result = result.wrapping_add\\(OCTAL_MASK_2\\);/        4 => {\\n            result = result.wrapping_add(OCTAL_FLAG);/'
  $'switch: default returns value instead of 0\ts/            result = 0;/            result = result;/'
  $'switch: arms 0 and 1 swapped\ts/match operation \\{\\n        0 =>/match operation {\\n        1 =>/'
  $'process_array_reverse: walks forward\ts/ptr = unsafe \\{ ptr.offset\\(-1\\) \\}/ptr = unsafe { ptr.offset(1) }/'
  $'process_array_reverse: count off by one\ts/    while i < count \\{\\n        sum = sum/    while i < count - 1 {\\n        sum = sum/'
  $'process_array_reverse: uses saturating add\ts/sum = sum.wrapping_add\\(unsafe \\{ \\*ptr \\}\\)/sum = sum.saturating_add(unsafe { *ptr })/'
  $'foreach_sum: skips the first element\ts/let mut idx: c_int = 0;/let mut idx: c_int = 1;/'
  $'foreach_sum: counts each element twice\ts/total = total.wrapping_add\\(element\\);/total = total.wrapping_add(element).wrapping_add(element);/'
  $'foreach_sum: <= instead of < in the guard\ts/while keep != 0 && idx < size/while keep != 0 \\&\\& idx <= size/'
  $'foreach_sum: uses saturating add\ts/total = total.wrapping_add\\(element\\);/total = total.saturating_add(element);/'
  $'allocate_and_compute: -1 becomes 0 on alloc failure\ts/    if points.is_null\\(\\) \\{\\n        return -1;/    if points.is_null() {\\n        return 0;/'
  $'allocate_and_compute: negative size clamped, not wrapped\ts/\\(size as isize as usize\\)/(size.max(0) as usize)/'
  $'allocate_and_compute: size cast as u32, not sign-extended\ts/\\(size as isize as usize\\)/(size as u32 as usize)/'
  $'allocate_and_compute: value field uses i not i*8\ts/\\(\\*p\\).value = i.wrapping_mul\\(OCTAL_BASE\\);/(*p).value = i;/'
  $'allocate_and_compute: coefficient uses i+1\ts/\\(\\*p\\).coefficient = \\(i as c_double\\) \\* multiplier;/(*p).coefficient = ((i + 1) as c_double) * multiplier;/'
  $'allocate_and_compute: struct size 12 not 16\ts/core::mem::size_of::<DataPoint>\\(\\)/12usize/'
  $'allocate_and_compute: sums in reverse order\ts/sum \\+= \\(\\*p\\).value as c_double \\* \\(\\*p\\).coefficient;/sum = (*p).value as c_double * (*p).coefficient + sum;/'
  $'allocate_and_compute: skips the last element\ts/    let mut i: c_int = 0;\\n    while i < size \\{\\n        unsafe \\{\\n            let p = points.offset\\(i as isize\\);\\n            sum/    let mut i: c_int = 0;\\n    while i < size - 1 {\\n        unsafe {\\n            let p = points.offset(i as isize);\\n            sum/'
)

# Mutations that are provably behaviour-preserving. They are expected to survive
# and are reported separately rather than counted as blind spots.
declare -A EQUIVALENT
EQUIVALENT["allocate_and_compute: sums in reverse order"]="a+b == b+a for IEEE-754 doubles (addition is commutative; only associativity differs, and the loop order is unchanged)"
EQUIVALENT["allocate_and_compute: size cast as u32, not sign-extended"]="unobservable through this API: for positive size the two casts are identical, and for EVERY negative int the u32 cast yields a byte request between 32.00 GiB (size=INT_MIN) and 63.99 GiB, all of which malloc refuses exactly like the correct 2^64-16 request -- so no int input distinguishes them. err_e9_alloc_fail_big asserts the 32 GiB precondition, so this claim breaks loudly if the host ever allows such an allocation. src/lib.rs already uses the exact C conversion (int -> size_t sign-extends)."

pass=0
fail=0
equiv=0
declare -a FAILED

for entry in "${MUTATIONS[@]}"; do
  name="${entry%%$'\t'*}"
  expr="${entry#*$'\t'}"
  cp "$BACKUP" "$SRC"
  if ! perl -0777 -i -pe "$expr" "$SRC" 2>/dev/null; then
    echo "PERL-ERROR $name"; fail=$((fail+1)); FAILED+=("$name"); continue
  fi
  if diff -q "$BACKUP" "$SRC" >/dev/null; then
    echo "NO-OP      $name  (pattern matched nothing -- mutation NOT applied)"
    fail=$((fail+1)); FAILED+=("$name"); continue
  fi
  if ! timeout 300 cargo build --lib --target-dir target/ffi-so >/dev/null 2>&1; then
    echo "NOCOMPILE  $name"; fail=$((fail+1)); FAILED+=("$name"); continue
  fi
  if timeout 300 cargo test --tests >/dev/null 2>&1; then
    if [ -n "${EQUIVALENT[$name]:-}" ]; then
      echo "equivalent $name  (${EQUIVALENT[$name]})"
      equiv=$((equiv+1))
    else
      echo "SURVIVED   $name  <-- suite is blind to this mutation"
      fail=$((fail+1)); FAILED+=("$name")
    fi
  else
    echo "caught     $name"
    pass=$((pass+1))
  fi
done

# Leave a correct, freshly built artifact behind.
cp "$BACKUP" "$SRC"
cargo build --lib --target-dir target/ffi-so >/dev/null 2>&1

echo
echo "caught: $pass   provably-equivalent: $equiv   blind spots: $fail   (total ${#MUTATIONS[@]})"
if [ "$fail" -ne 0 ]; then
  echo "blind spots:"
  printf '  - %s\n' "${FAILED[@]}"
fi
[ "$fail" -eq 0 ]
