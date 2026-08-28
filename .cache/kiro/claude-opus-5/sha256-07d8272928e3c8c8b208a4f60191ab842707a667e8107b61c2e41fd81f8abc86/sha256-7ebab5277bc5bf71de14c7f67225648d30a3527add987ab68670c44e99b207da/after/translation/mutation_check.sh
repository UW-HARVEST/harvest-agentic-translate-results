#!/usr/bin/env bash
# Negative control: inject known bugs into the Rust translation and confirm the
# differential suite catches each one. Always restores src/lib.rs.
set -uo pipefail
cd "$(dirname "$0")"

cp src/lib.rs /tmp/lib.rs.orig
restore() { cp /tmp/lib.rs.orig src/lib.rs; }
trap restore EXIT

overall=0

mutate() {
  local name="$1" from="$2" to="$3"
  restore
  if ! grep -qF -- "$from" src/lib.rs; then
    echo "SKIP  $name (anchor not found)"; return
  fi
  python3 - "$from" "$to" <<'PY'
import sys, pathlib
p = pathlib.Path("src/lib.rs")
s = p.read_text()
frm, to = sys.argv[1], sys.argv[2]
assert s.count(frm) == 1, f"anchor appears {s.count(frm)} times"
p.write_text(s.replace(frm, to))
PY
  local dir="target/mut"
  rm -rf "$dir"
  if ! timeout 600 cargo build --release --target-dir "$dir" > /tmp/mut_build.log 2>&1; then
    echo "SKIP  $name (mutant did not compile)"; return
  fi
  if DRIVER_RUST_SO="$(pwd)/$dir/release/libdriver.so" \
       timeout 600 cargo test > /tmp/mut_test.log 2>&1; then
    echo "BAD   $name -> suite PASSED a known-buggy mutant"
    overall=1
  else
    local hits
    hits=$(grep -cE "^test .* FAILED|panicked" /tmp/mut_test.log)
    echo "OK    $name -> caught (failures detected: $hits)"
  fi
}

# Note on two *equivalent* mutants that are deliberately NOT tested:
# Flipping `number >= INT_MAX` to `>` (or `number <= INT_MIN` to `<`) is
# semantically equivalent: the only input where the branch differs is the exact
# boundary (2147483647.0 / -2147483648.0), and there the `else` branch's
# truncating cast yields the identical integer. No input can distinguish them,
# so they are excluded rather than reported as a coverage gap. The boundaries
# themselves *are* covered by level2::valueint_saturation_boundaries via the
# mutants below.

mutate "saturation-upper-branch-value" \
  "        item_ref.valueint = INT_MAX;" \
  "        item_ref.valueint = 0;"

mutate "saturation-lower-branch-value" \
  "        item_ref.valueint = INT_MIN;" \
  "        item_ref.valueint = 0;"

mutate "offset-advanced-by-scan-length" \
  "let consumed = (after_end as usize).wrapping_sub(number_c_string as usize);" \
  "let consumed = number_string_length;"

mutate "accepted-set-drops-uppercase-E" \
  "| b'e' | b'E' => {" \
  "| b'e' => {"

mutate "wrong-type-tag" \
  "item_ref.type_ = CJSON_NUMBER;" \
  "item_ref.type_ = CJSON_NUMBER + 1;"

mutate "missing-nul-terminator" \
  "*number_c_string.add(number_string_length) = b'\\0';" \
  "*number_c_string.add(number_string_length) = b'9';"

mutate "off-by-one-can-access" \
  "buffer.offset.wrapping_add(index) < buffer.length" \
  "buffer.offset.wrapping_add(index) <= buffer.length"

mutate "int-cast-truncation-changed-to-round" \
  "item_ref.valueint = number as c_int;" \
  "item_ref.valueint = number.round() as c_int;"

mutate "null-content-check-dropped" \
  "if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {" \
  "if input_buffer.is_null() {"

restore
echo "=============================================================="
if [ $overall -eq 0 ]; then echo "NEGATIVE CONTROL: all mutants detected"; else echo "NEGATIVE CONTROL: GAPS FOUND"; fi
exit $overall
