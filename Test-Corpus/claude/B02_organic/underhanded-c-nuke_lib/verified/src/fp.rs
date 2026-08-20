// x86-64 SSE scalar arithmetic helpers that reproduce the hardware's *NaN
// operand selection*, which is observable whenever two NaNs with different
// payloads meet.
//
// Intel SDM, "Operation" tables for ADDSD/MULSD/MULSS etc.:
//
//     if SRC1 is NaN  -> result is QNaN(SRC1)
//     elif SRC2 is NaN -> result is QNaN(SRC2)
//     else            -> the arithmetic result
//
// where `SRC1` is the *destination* register of the two-operand form
// (`addsd %xmm_src2, %xmm_src1` computes `src1 = src1 + src2`) and `QNaN(x)`
// is `x` with the mantissa MSB (the quiet bit) forced on.
//
// This matters because gcc, for every `sum += X` in the C sources, loads the
// *new element* into the destination register and the accumulator into the
// source register:
//
//     movsd  X,      %xmm0     ; SRC1 / dest = X
//     movsd  sum,    %xmm1     ; SRC2       = sum
//     addsd  %xmm1,  %xmm0     ; xmm0 = X + sum
//
// so the C result inherits the payload of `X`, whereas a plain Rust
// `sum += x` compiles to `addsd %x, %sum` and would inherit the payload of
// `sum`.  Likewise `a[i] * b[i]` in `dot_product` is emitted as
// `mulss %xmm1(=a[i]), %xmm0(=b[i])`, i.e. `b[i]` is SRC1.
//
// Only the commutative operations need these helpers: for `subsd` and `divsd`
// the operand roles are already pinned by the arithmetic itself, so plain Rust
// `-` and `/` map onto the same instruction with the same SRC1.

/// `x` with the quiet bit forced on (the `QNaN(x)` of the SDM tables).
#[inline]
fn quiet_f64(x: f64) -> f64 {
    f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

/// `x` with the quiet bit forced on.
#[inline]
fn quiet_f32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `addsd src2, src1` -> `src1 + src2`, with x86 NaN operand selection.
///
/// When neither operand is NaN the plain `+` is used, so an invalid operation
/// such as `+inf + -inf` still produces the hardware's default QNaN
/// (`0xFFF8_0000_0000_0000`) exactly as the C does.
#[inline]
pub(crate) fn addsd(src1: f64, src2: f64) -> f64 {
    if src1.is_nan() {
        quiet_f64(src1)
    } else if src2.is_nan() {
        quiet_f64(src2)
    } else {
        src1 + src2
    }
}

/// `mulsd src2, src1` -> `src1 * src2`, with x86 NaN operand selection.
#[inline]
pub(crate) fn mulsd(src1: f64, src2: f64) -> f64 {
    if src1.is_nan() {
        quiet_f64(src1)
    } else if src2.is_nan() {
        quiet_f64(src2)
    } else {
        src1 * src2
    }
}

/// `mulss src2, src1` -> `src1 * src2`, with x86 NaN operand selection.
#[inline]
pub(crate) fn mulss(src1: f32, src2: f32) -> f32 {
    if src1.is_nan() {
        quiet_f32(src1)
    } else if src2.is_nan() {
        quiet_f32(src2)
    } else {
        src1 * src2
    }
}
