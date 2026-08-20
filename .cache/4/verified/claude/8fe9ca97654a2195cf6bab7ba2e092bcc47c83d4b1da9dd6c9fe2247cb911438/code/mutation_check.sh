#!/bin/sh
# Non-vacuity check for the differential suite: inject one small mutation into
# the Rust translation at a time, confirm the suite CATCHES it, then restore.
#
# A differential test suite that passes is only meaningful if it would also fail;
# this is the evidence for that.  Usage: ./mutation_check.sh
set -e
cd "$(dirname "$0")"

BK=$(mktemp -d "${TMPDIR:-/tmp}/mut.XXXXXX")
cp src/q_math.rs src/q_shared.rs src/cstd.rs "$BK/"
restore() {
    cp "$BK/q_math.rs" "$BK/q_shared.rs" "$BK/cstd.rs" src/
    cargo build --offline > /dev/null 2>&1 || true
}
trap 'restore; rm -rf "$BK"' EXIT INT TERM

# mutation: file, sed expression, the test target that must fail
run_mutation() {
    desc="$1"; file="$2"; expr="$3"; target="$4"
    cp "$BK/$(basename "$file")" "$file"
    before=$(md5sum "$file" | cut -d' ' -f1)
    sed -i "$expr" "$file"
    after=$(md5sum "$file" | cut -d' ' -f1)
    if [ "$before" = "$after" ]; then
        echo "SETUP ERROR: mutation '$desc' did not change $file"
        exit 1
    fi
    cargo build --offline > /dev/null 2>&1
    if timeout 600 cargo test --offline --test "$target" > /dev/null 2>&1; then
        echo "NOT CAUGHT: $desc  (tests/$target.rs still passes!)"
        status=1
    else
        echo "caught by tests/$target.rs: $desc"
    fi
    cp "$BK/$(basename "$file")" "$file"
}

status=0
run_mutation "Q_rsqrt magic constant 0x5f3759df -> 0x5f3759de" \
    src/q_math.rs 's/0x5f3759dfu32/0x5f3759deu32/' scalar
run_mutation "AngleMod rounding: 65536.0/360.0 -> 65536.0/360.1" \
    src/q_math.rs 's|(65536.0f64 / 360.0f64)|(65536.0f64 / 360.1f64)|' scalar
run_mutation "cvttsd2si emulation replaced by Rust saturating cast" \
    src/q_shared.rs 's/if d.is_nan() || d >= 2147483648.0f64 || d < -2147483648.0f64 {/if false {/' scalar
run_mutation "ClampChar boundary -128 -> -127" \
    src/q_math.rs 's/        return -128;/        return -127;/' scalar
# NOTE: `(x as f64).sqrt() as f32` and `x.sqrt()` are provably the same value for
# every f32 (double rounding is innocuous for sqrt when 53 >= 2*24+2), so that is
# NOT a usable mutation.  Perturb the scaling instead.
run_mutation "VectorNormalize: scale by ilength+0 instead of ilength" \
    src/q_math.rs 's|        \*v.add(0) = \*v.add(0) \* ilength;|        *v.add(0) = *v.add(0) * (ilength + 1e-9);|' vectors
run_mutation "MatrixMultiply: swap two products" \
    src/q_math.rs 's/    (\*out.add(0))\[1\] = a(0, 0) \* b(0, 1)/    (*out.add(0))[1] = a(0, 1) * b(0, 1)/' vectors
run_mutation "DirToByte: > becomes >=" \
    src/q_math.rs 's/        if d > bestd {/        if d >= bestd {/' planes
run_mutation "SetPlaneSignbits: -0.0 counted as negative" \
    src/q_math.rs 's|        if (\*out).normal\[j\] < 0.0 {|        if (*out).normal[j].is_sign_negative() {|' planes
run_mutation "AngleVectors: sin\/cos in f32" \
    src/q_math.rs 's/    sy = (angle as f64).sin() as f32;/    sy = angle.sin();/' angles
run_mutation "RAD2DEG: multiply in f64 instead of f32" \
    src/q_shared.rs 's|(a \* 180.0f32) as f64 / M_PI|a as f64 * 180.0f64 / M_PI|' qshared
run_mutation "bytedirs table: perturb one entry" \
    src/q_math.rs 's/    \[-0.525731f32, 0.000000f32, 0.850651f32\],/    [-0.525732f32, 0.000000f32, 0.850651f32],/' data
run_mutation "atof: mis-parse a hex float" \
    src/cstd.rs 's/    if i + 1 < s.len() \&\& s\[i\] == b.0. \&\& (s\[i + 1\] | 0x20) == b.x. {/    if false {/' driver_cli

restore
echo
if [ "$status" = 0 ]; then
    echo "ALL MUTATIONS CAUGHT -- the suite is not vacuous"
else
    echo "SOME MUTATIONS SURVIVED (see above)"
fi
exit "$status"
