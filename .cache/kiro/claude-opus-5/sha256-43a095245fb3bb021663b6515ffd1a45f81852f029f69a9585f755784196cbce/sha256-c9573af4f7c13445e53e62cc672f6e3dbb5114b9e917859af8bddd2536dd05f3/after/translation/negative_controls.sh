#!/usr/bin/env bash
# Negative controls: deliberately break the Rust translation in many ways and
# confirm the FULL differential suite catches each break. A suite that passes but
# cannot fail is worthless. src/lib.rs is restored afterwards.
#
# Each mutant is checked against the entire suite (not a hand-picked test), so a
# "detected" result means the suite as a whole is sensitive to that behaviour.
set -u
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

fail=0
mutate() { # name  perl-expr
  local name="$1" expr="$2"
  cp "$BAK" src/lib.rs
  perl -0pi -e "$expr" src/lib.rs
  if cmp -s "$BAK" src/lib.rs; then
    echo "NO-OP MUTATION  $name (pattern matched nothing)"; fail=1; return
  fi
  if ! timeout 300 cargo build --release -q 2>/dev/null; then
    echo "BUILD-FAILED    $name"; fail=1; return
  fi
  if timeout 400 cargo test --release -q >/dev/null 2>&1; then
    echo "NOT DETECTED    $name  <-- suite is blind here"
    fail=1
  else
    echo "detected        $name"
  fi
}

# --- safe_double_to_int -----------------------------------------------------
mutate "sdti: NaN -> 1 instead of 0"        's/\} else if d\.is_nan\(\) \{\n        0\n/} else if d.is_nan() {\n        1\n/'
mutate "sdti: high guard returns INT_MIN"   's/if d > INT_MAX as c_double \{\n        INT_MAX/if d > INT_MAX as c_double {\n        INT_MIN/'
mutate "sdti: low guard returns INT_MAX"    's/\} else if d < INT_MIN as c_double \{\n        INT_MIN/} else if d < INT_MIN as c_double {\n        INT_MAX/'
mutate "sdti: high guard compares < not >"  's/if d > INT_MAX as c_double/if d < INT_MAX as c_double/'
mutate "sdti: NaN check moved before range" 's/if d > INT_MAX as c_double \{\n        INT_MAX\n    \} else if d < INT_MIN as c_double \{\n        INT_MIN\n    \} else if d\.is_nan\(\) \{\n        0/if d.is_nan() {\n        1\n    } else if d > INT_MAX as c_double {\n        INT_MAX\n    } else if d < INT_MIN as c_double {\n        INT_MIN/'
mutate "sdti: cast rounds instead of trunc" 's/        d as c_int\n/        d.round() as c_int\n/'

# --- process_with_fallthrough ----------------------------------------------
mutate "pwf: default -1 -> 0"               's/            result = -1;/            result = 0;/'
mutate "pwf: case 5 drops the +50"          's/            result = result\.wrapping_add\(50\);\n/            /'
mutate "pwf: case 2 stops falling through"  's/        2 => \{\n            result = result\.wrapping_add\(20\);\n            result = result\.wrapping_add\(10\);/        2 => {\n            result = result.wrapping_add(20);/'
mutate "pwf: case 0 keeps base_value"       's/        0 => \{\n            result = 0;/        0 => {\n            result = result;/'
mutate "pwf: case 6 also accepted"          's/        5 => \{/        5 | 6 => {/'

# --- copy_data_block --------------------------------------------------------
mutate "copy: copies 1 byte fewer"          's/std::mem::size_of::<DataBlock>\(\),\n        \);\n    \}\n\}/std::mem::size_of::<DataBlock>() - 1,\n        );\n    }\n}/'
mutate "copy: adds a null check"            's/pub unsafe extern "C" fn copy_data_block\(dest: \*mut DataBlock, src: \*const DataBlock\) \{\n    unsafe \{/pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {\n    if dest.is_null() || src.is_null() { return; }\n    unsafe {/'

# --- handle_pointer_operations ---------------------------------------------
mutate "hpo: +100 -> +101"                  's/\(\*ptr\)\.wrapping_add\(100\)/(*ptr).wrapping_add(101)/'
mutate "hpo: *2 -> *3"                      's/value\.wrapping_mul\(2\)/value.wrapping_mul(3)/'

# --- overunder --------------------------------------------------------------
mutate "overunder: printf %.2f -> %.3f"     's/value=%\.2f/value=%.3f/'
mutate "overunder: label \"Source\"->\"Sourcf\"" 's/let src = b"Source";/let src = b"Sourcf";/'
mutate "overunder: prints result_9 label"   's/result_2 = %d/result_9 = %d/'
mutate "overunder: array slot 4 = a-b"      's/\[a, b, c, d, a\.wrapping_add\(b\)\]/[a, b, c, d, a.wrapping_sub(b)]/'
mutate "overunder: total drops dest id"     's/total = total\.wrapping_add\(dest_block\.id\);/let _ = dest_block.id;/'
mutate "overunder: sqrt operand as i64"     's/\(d\.wrapping_mul\(d\)\.wrapping_add\(a\.wrapping_mul\(a\)\)\) as c_double/((d as i64 * d as i64) + (a as i64 * a as i64)) as c_double/'
mutate "overunder: a % 6 -> rem_euclid"     's/process_with_fallthrough\(a % 6, b\)/process_with_fallthrough(a.rem_euclid(6), b)/'
mutate "overunder: temp2 uses 2.6 not 2.7"  's/b as c_double \* 2\.7/b as c_double * 2.6/'
mutate "overunder: temp3 uses 3.2 not 3.3"  's/c as c_double \/ 3\.3/c as c_double \/ 3.2/'
mutate "overunder: temp1 uses 1.4 not 1.5"  's/a as c_double \* 1\.5/a as c_double * 1.4/'
mutate "overunder: hpo called with d not c" 's/handle_pointer_operations\(c\);/handle_pointer_operations(d);/'
mutate "overunder: label not NUL-padded"    's/        while i < n \{\n            label\[i\] = 0;\n            i \+= 1;\n        \}\n/        while i < n {\n            label[i] = 0x41;\n            i += 1;\n        }\n/'

restore
timeout 300 cargo build --release -q 2>/dev/null

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL NEGATIVE CONTROLS DETECTED"
else
  echo "SOME NEGATIVE CONTROLS NOT DETECTED"
fi
exit "$fail"
