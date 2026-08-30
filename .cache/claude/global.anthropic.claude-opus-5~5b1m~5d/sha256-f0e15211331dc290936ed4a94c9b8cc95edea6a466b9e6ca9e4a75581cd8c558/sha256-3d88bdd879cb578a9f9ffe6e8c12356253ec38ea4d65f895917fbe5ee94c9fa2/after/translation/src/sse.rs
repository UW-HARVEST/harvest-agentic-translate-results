//! Models the x86-64 SSE scalar single-precision operations that the C
//! compiler emits, including their NaN propagation rules:
//!
//! * `OP dst, src` returns `dst`'s NaN (quieted) if `dst` is a NaN,
//!   otherwise `src`'s NaN (quieted) if `src` is a NaN.
//! * An invalid operation on non-NaN operands (inf - inf, 0 * inf) produces
//!   the "indefinite" QNaN `0xFFC00000`.
//!
//! Spelling the operations out this way keeps the results independent of the
//! Rust optimizer, which is otherwise free to alter the sign of NaN results.

/// x86 "real indefinite" QNaN.
const INDEFINITE: u32 = 0xFFC0_0000;

fn indefinite() -> f32 {
    f32::from_bits(INDEFINITE)
}

fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `addss dst, src`
pub fn addss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet(dst);
    }
    if src.is_nan() {
        return quiet(src);
    }
    if dst.is_infinite() && src.is_infinite() && dst.is_sign_negative() != src.is_sign_negative() {
        return indefinite();
    }
    dst + src
}

/// `subss dst, src`
pub fn subss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet(dst);
    }
    if src.is_nan() {
        return quiet(src);
    }
    if dst.is_infinite() && src.is_infinite() && dst.is_sign_negative() == src.is_sign_negative() {
        return indefinite();
    }
    dst - src
}

/// `mulss dst, src`
pub fn mulss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet(dst);
    }
    if src.is_nan() {
        return quiet(src);
    }
    if (dst == 0.0 && src.is_infinite()) || (src == 0.0 && dst.is_infinite()) {
        return indefinite();
    }
    dst * src
}

/// `andps` with a 0x7FFFFFFF mask, i.e. C's `fabsf`.
pub fn fabs(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}
