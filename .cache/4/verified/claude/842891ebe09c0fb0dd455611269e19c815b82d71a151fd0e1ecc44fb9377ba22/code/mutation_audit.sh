#!/usr/bin/env bash
# Mutation-sensitivity audit for the differential test suite.
#
# Each mutation injects a *plausible* C-to-Rust translation bug into
# src/lib.rs; the suite must FAIL for every semantically-observable one.
# A mutation that survives means the suite has a blind spot there.
#
# Usage: ./mutation_audit.sh
set -uo pipefail
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap restore EXIT

# name | python-replacement-expression
run_mutation() {
  local name="$1" from="$2" to="$3"
  restore
  python3 - "$from" "$to" <<'PY'
import sys
frm, to = sys.argv[1], sys.argv[2]
p = 'src/lib.rs'
s = open(p).read()
assert frm in s, f"mutation pattern not found: {frm!r}"
open(p, 'w').write(s.replace(frm, to))
PY
  if [ $? -ne 0 ]; then echo "SETUP-FAIL  $name"; return; fi

  out=$(timeout 600 cargo test 2>&1)
  # Order matters: `cargo test` prints "error: test failed, to rerun pass ..."
  # on assertion failures, which must NOT be classified as a compile error.
  if echo "$out" | grep -qE "^test .* FAILED|panicked at"; then
    local n
    n=$(echo "$out" | grep -c "^test .* FAILED")
    local first
    first=$(echo "$out" | grep -m1 "^test .* FAILED" | sed 's/ \.\.\..*//; s/^test //')
    echo "CAUGHT      $name  ($n failing tests, e.g. $first)"
  elif echo "$out" | grep -qE "error: could not compile|error\[E[0-9]+\]"; then
    echo "COMPILE-ERR $name  (mutation not type-correct; rewrite it)"
  else
    echo "SURVIVED    $name   <-- BLIND SPOT"
  fi
}

# Same, but applies several `from=>to` replacements at once, for mutations that
# are only type-correct when applied together.
run_mutation_multi() {
  local name="$1"; shift
  restore
  python3 - "$@" <<'PY'
import sys
p = 'src/lib.rs'
s = open(p).read()
for spec in sys.argv[1:]:
    frm, to = spec.split('=>', 1)
    assert frm in s, f"mutation pattern not found: {frm!r}"
    s = s.replace(frm, to)
open(p, 'w').write(s)
PY
  if [ $? -ne 0 ]; then echo "SETUP-FAIL  $name"; return; fi
  out=$(timeout 600 cargo test 2>&1)
  if echo "$out" | grep -qE "^test .* FAILED|panicked at"; then
    local n first
    n=$(echo "$out" | grep -c "^test .* FAILED")
    first=$(echo "$out" | grep -m1 "^test .* FAILED" | sed 's/ \.\.\..*//; s/^test //')
    echo "CAUGHT      $name  ($n failing tests, e.g. $first)"
  elif echo "$out" | grep -qE "error: could not compile|error\[E[0-9]+\]"; then
    echo "COMPILE-ERR $name  (mutation not type-correct; rewrite it)"
  else
    echo "SURVIVED    $name   <-- BLIND SPOT"
  fi
}

echo "=== baseline (unmutated: must pass) ==="
restore
if timeout 600 cargo test 2>&1 | grep -q "FAILED"; then
  echo "BASELINE FAILS — fix that first"; exit 1
else
  echo "baseline OK"
fi

echo
echo "=== mutations ==="

# --- luminance coefficients (lines 9 of the C) ---
run_mutation "coeff R<->G swapped" \
  "0.2126f32 * r + 0.7152f32 * g" "0.7152f32 * r + 0.2126f32 * g"
run_mutation "coeff G<->B swapped" \
  "0.7152f32 * g + 0.0722f32 * b" "0.0722f32 * g + 0.7152f32 * b"
run_mutation "coeff typo 0.2126 -> 0.2125" \
  "0.2126f32 * r" "0.2125f32 * r"
run_mutation "coeff computed in f64" \
  "let result = 0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b;" \
  "let result = (0.2126f64 * r as f64 + 0.7152f64 * g as f64 + 0.0722f64 * b as f64) as f32;"
run_mutation "dot product contracted to FMA" \
  "let result = 0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b;" \
  "let result = 0.2126f32.mul_add(r, 0.7152f32.mul_add(g, 0.0722f32 * b));"
run_mutation "dot product right-associated" \
  "let result = 0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b;" \
  "let result = 0.2126f32 * r + (0.7152f32 * g + 0.0722f32 * b);"

# --- sRGB linearization (lines 6-8 of the C) ---
run_mutation "linearize entirely in f32" \
  "let c = c as f64;
    let v = if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    };
    v as f32" \
  "let v = if c > 0.04045f32 {
        ((c + 0.055f32) / 1.055f32).powf(2.4f32)
    } else {
        c / 12.92f32
    };
    v"
# The C casts the ternary result back to `float` on every channel BEFORE the dot
# product; keeping f64 all the way through is the classic translation slip.
run_mutation_multi "no f64->f32 truncation (per-channel f64 kept)" \
  "fn cb_linearize(c: f32) -> f32 {=>fn cb_linearize(c: f32) -> f64 {" \
  "    v as f32
}=>    v
}" \
  "let result = 0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b;=>let result = (0.2126f32 as f64 * r + 0.7152f32 as f64 * g + 0.0722f32 as f64 * b) as f32;"
# NOTE: provably equivalent, kept as a control. No u8 n has n/255.f in
# (0.04045, 0.0405], so no input can distinguish the two thresholds.
run_mutation "[control, provably equivalent] threshold 0.04045 -> 0.0405" \
  "c > 0.04045" "c > 0.0405"
run_mutation "threshold 0.04045 -> 0.03" "c > 0.04045" "c > 0.03"
run_mutation "exponent 2.4 -> 2.2" "powf(2.4)" "powf(2.2)"
run_mutation "offset 0.055 -> 0.05" "(c + 0.055)" "(c + 0.05)"
run_mutation "divisor 1.055 -> 1.05" "/ 1.055)" "/ 1.05)"
run_mutation "divisor 12.92 -> 12.9" "c / 12.92" "c / 12.9"
run_mutation "branches inverted" \
  "if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }" \
  "if c > 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }"
run_mutation "powf -> exp/ln reimplementation" \
  "((c + 0.055) / 1.055).powf(2.4)" \
  "(2.4 * ((c + 0.055) / 1.055).ln()).exp()"

# --- the High/Low swap (lines 17-20 of the C) ---
run_mutation "swap removed" \
  "if high < low {
        high = lum_b;
        low = lum_a;
    }" \
  "if false {
        high = lum_b;
        low = lum_a;
    }"
run_mutation "swap always taken" \
  "if high < low {" "if true {"
run_mutation "swap condition inverted (>)" "if high < low {" "if high > low {"
run_mutation "ratio inverted (low/high)" "let ratio = high / low;" "let ratio = low / high;"
run_mutation "div-by-zero 'guard' added" \
  "let ratio = high / low;" \
  "let ratio = if low == 0.0 { 0.0 } else { high / low };"
run_mutation "epsilon clamp added to denominator" \
  "let ratio = high / low;" \
  "let ratio = high / low.max(1e-7);"

# --- the u8 -> f32 conversions and struct layout (line 26-28 / lib.h) ---
run_mutation "divide by 256 instead of 255" \
  "f32::from(A.R) / 255.0f32" "f32::from(A.R) / 256.0f32"
# NOTE: provably equivalent, kept as a control. Verified for all 256 values of
# n that f32(n/255 computed in f64) == f32(n)/f32(255), so double rounding never
# shows up here.
run_mutation "[control, provably equivalent] channel conversion via f64" \
  "f32::from(A.G) / 255.0f32" "((A.G as f64) / 255.0f64) as f32"
run_mutation "struct field order R/G/B -> B/G/R" \
  "pub R: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub B: core::ffi::c_uchar," \
  "pub B: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub R: core::ffi::c_uchar,"
run_mutation "A.R/A.B transposed at the call site" \
  "        f32::from(A.R) / 255.0f32,
        f32::from(A.G) / 255.0f32,
        f32::from(A.B) / 255.0f32," \
  "        f32::from(A.B) / 255.0f32,
        f32::from(A.G) / 255.0f32,
        f32::from(A.R) / 255.0f32,"
run_mutation "B channels used for A" \
  "        f32::from(A.R) / 255.0f32,
        f32::from(A.G) / 255.0f32,
        f32::from(A.B) / 255.0f32,
        f32::from(B.R) / 255.0f32," \
  "        f32::from(B.R) / 255.0f32,
        f32::from(A.G) / 255.0f32,
        f32::from(A.B) / 255.0f32,
        f32::from(B.R) / 255.0f32,"
# Real ABI break: 12-byte struct is MEMORY-class under SysV, not a single
# INTEGER eightbyte, so it is passed completely differently from the C's 3-byte
# struct. Catching this proves the harness really tests the FFI ABI.
run_mutation "struct widened to u16 fields (field offsets change)" \
  "pub struct cb_rgb_255 {
    pub R: core::ffi::c_uchar,
    pub G: core::ffi::c_uchar,
    pub B: core::ffi::c_uchar,
}" \
  "pub struct cb_rgb_255 {
    pub R: u16,
    pub G: u16,
    pub B: u16,
}"
# NOTE: provably equivalent, kept as a control. rustc's default (repr(Rust))
# layout only reorders fields to reduce padding; with three fields of identical
# type/alignment there is nothing to reorder, so the layout and the SysV
# argument classification coincide with repr(C). The shipped code keeps repr(C),
# which is what makes that guaranteed rather than incidental.
run_mutation "[control, provably equivalent] repr(C) removed from the struct" \
  "#[repr(C)]
#[derive(Clone, Copy)]
pub struct cb_rgb_255 {" \
  "#[derive(Clone, Copy)]
pub struct cb_rgb_255 {"
# Real ABI break: taking the structs by pointer instead of by value.
run_mutation "extern \"C\" changed to the Rust ABI" \
  'pub extern "C" fn contrast_ratio' \
  'pub extern "sysv64" fn contrast_ratio_renamed_probe'

# --- export surface ---
run_mutation "no_mangle removed (symbol disappears)" \
  '#[unsafe(no_mangle)]' \
  '#[cfg_attr(any(), unsafe(no_mangle))]'

restore
echo
echo "=== done — restoring original source ==="
