//! Explicit x86 SSE NaN-propagation semantics for the *commutative* float ops.
//!
//! # Why this exists
//!
//! For `addss`/`mulss`/`addps`/`mulps`, when **both** operands are NaN the hardware
//! returns the *destination* operand (quieted); when only one is NaN it returns that
//! one (quieted). Since `+` and `*` are commutative, a compiler is free to pick
//! either operand as the destination, and GCC and LLVM make different choices. That
//! is invisible for ordinary values but selects a different NaN sign/payload when two
//! NaNs meet in one instruction — a real, reachable difference through the public API
//! (e.g. `omni_manifold` with `inf`/NaN coordinates).
//!
//! `mul`/`add` below take the destination operand *first*, so the choice GCC made for
//! the C library is recorded in the source instead of being left to the register
//! allocator. Non-commutative ops (`-`, `/`) need no helper: their destination is
//! forced to the left operand, so plain `-` and `/` already agree.
//!
//! Reference: Intel SDM, "Operation" pseudocode for MULSS/ADDSS — the QNaN result of
//! an operation with a NaN source is the *first* (destination) NaN operand, quieted.
//!
//! # Where the destination choices come from
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference library is
//! compiled at **`-O0`**. Every operand order in this crate was read off
//! `objdump -d c_src/build/libtranslated_rust.so` and is quoted in a comment at the
//! call site. At `-O0` GCC's destination is *not* consistently the left operand — it
//! falls out of the order the operands happen to be loaded in — so several sites are
//! reversed relative to the C source. For example `c2Dot`:
//!
//! ```text
//!   movss -0x8(%rbp),%xmm1   ; a.x
//!   movss -0x10(%rbp),%xmm0  ; b.x
//!   mulss %xmm0,%xmm1        ; dst = a.x
//!   movss -0x4(%rbp),%xmm2   ; a.y
//!   movss -0xc(%rbp),%xmm0   ; b.y
//!   mulss %xmm2,%xmm0        ; dst = b.y   <-- opposite choice from the x lane
//!   addss %xmm1,%xmm0        ; dst = the y product
//! ```
//!
//! i.e. `add(mul(b.y, a.y), mul(a.x, b.x))`. Getting any of these backwards is
//! invisible for ordinary values and shows up only when two NaNs meet in one
//! instruction — which the differential tests in `tests/phase_b_primitives.rs` do
//! exhaustively over a 26-value pool of special floats.
//!
//! Note that `-O0` is only how the *reference* is built; this crate is verified in
//! both `cargo test` and `cargo test --release`, so LLVM's optimiser is not permitted
//! to collapse the selects below either.

/// Set the quiet bit, exactly as the hardware does when it propagates a NaN.
/// Identity for a NaN that is already quiet.
#[inline(always)]
fn quiet(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// `mulss dst, src` — i.e. `dst * src`, with `dst`'s NaN taking precedence.
///
/// The product is computed unconditionally and the bit-twiddling select is done on
/// integer operations so that LLVM cannot collapse the branch by treating the two
/// NaN results as interchangeable.
#[inline(always)]
pub fn mul(dst: f32, src: f32) -> f32 {
    let p = dst * src;
    if dst.is_nan() { quiet(dst) } else { p }
}

/// `addss dst, src` — i.e. `dst + src`, with `dst`'s NaN taking precedence.
#[inline(always)]
pub fn add(dst: f32, src: f32) -> f32 {
    let s = dst + src;
    if dst.is_nan() { quiet(dst) } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;

    // Every operand goes through `black_box`. Const-evaluated float arithmetic does
    // not model hardware NaN-payload propagation (it yields the default NaN
    // 0x7fc00000), so folded operands would test the wrong thing. In the library
    // itself the operands always come from function arguments, i.e. never folded.
    fn pq() -> f32 {
        black_box(f32::from_bits(0x7fc0_1234)) // +qNaN, distinctive payload
    }
    fn nq() -> f32 {
        black_box(f32::from_bits(0xffc0_5678)) // -qNaN, distinctive payload
    }
    fn ps() -> f32 {
        black_box(f32::from_bits(0x7f80_0001)) // +sNaN
    }

    #[test]
    fn dst_nan_wins_over_src_nan() {
        assert_eq!(mul(pq(), nq()).to_bits(), pq().to_bits());
        assert_eq!(mul(nq(), pq()).to_bits(), nq().to_bits());
        assert_eq!(add(pq(), nq()).to_bits(), pq().to_bits());
        assert_eq!(add(nq(), pq()).to_bits(), nq().to_bits());
    }

    #[test]
    fn lone_src_nan_propagates() {
        assert_eq!(mul(black_box(2.0), nq()).to_bits(), nq().to_bits());
        assert_eq!(add(black_box(2.0), nq()).to_bits(), nq().to_bits());
    }

    #[test]
    fn signaling_nan_is_quieted() {
        assert_eq!(mul(ps(), black_box(1.0)).to_bits(), 0x7fc0_0001);
        assert_eq!(add(ps(), black_box(1.0)).to_bits(), 0x7fc0_0001);
    }

    #[test]
    fn ordinary_values_unaffected() {
        assert_eq!(mul(black_box(3.0), black_box(4.0)), 12.0);
        assert_eq!(add(black_box(3.0), black_box(4.0)), 7.0);
        // signed zero behaviour must match plain arithmetic
        let (nz, pz) = (black_box(-0.0f32), black_box(0.0f32));
        assert_eq!(add(nz, pz).to_bits(), (nz + pz).to_bits());
        assert_eq!(mul(nz, black_box(1.0f32)).to_bits(), (nz * 1.0f32).to_bits());
    }
}
