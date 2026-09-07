#!/usr/bin/env python3
"""Regenerate translation/src/lib.rs mechanically from c_src/src/lib.c.

Parses the three `static` lookup tables straight out of the C source so the
Rust tables cannot drift from the C by construction, then emits the translated
`half2float`.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
C_SRC = ROOT / "c_src" / "src" / "lib.c"
OUT = ROOT / "translation" / "src" / "lib.rs"

src = C_SRC.read_text()


def table(decl_regex, width):
    """Extract the initializer list of a C table declaration as hex literals."""
    m = re.search(decl_regex + r"\s*=\s*\{(.*?)\};", src, re.S)
    if not m:
        sys.exit(f"could not find table matching {decl_regex}")
    vals = re.findall(r"0x[0-9a-fA-F]+", m.group(1))
    return [f"0x{int(v, 16):0{width}x}" for v in vals]


mant = table(r"static\s+uint32_t\s+m__mantissa\s*\[\s*2048\s*\]", 8)
off = table(r"static\s+uint16_t\s+m__offset\s*\[\s*64\s*\]", 4)
exp = table(r"static\s+uint32_t\s+m__exponent\s*\[\s*64\s*\]", 8)

assert len(mant) == 2048, f"mantissa: got {len(mant)}"
assert len(off) == 64, f"offset: got {len(off)}"
assert len(exp) == 64, f"exponent: got {len(exp)}"


def emit(name, ty, vals, per_line):
    lines = [f"static {name}: [{ty}; {len(vals)}] = ["]
    for i in range(0, len(vals), per_line):
        lines.append("    " + ", ".join(vals[i : i + per_line]) + ",")
    lines.append("];")
    return "\n".join(lines)


HEADER = '''//! Rust translation of c_src/src/lib.c
//!
//! Public ABI: `float half2float(uint16_t h)`
//!
//! The three lookup tables below are extracted mechanically from the C source
//! by tools/gen_lib_rs.py, so the conversion is bit-for-bit identical.

#![allow(non_upper_case_globals)]

use std::ffi::{c_float, c_uint};
'''

BODY = '''
/// The literal translation of the C body, for a `h` already narrowed to 16 bits.
///
/// `n` is in 0..=63 because `h` is 16 bits, and `(h & 0x3ff) + m__offset[n]` is
/// at most 0x3ff + 0x400 = 0x7ff, so both table accesses are in bounds. The
/// masks make that provable to the compiler, so no panic path is emitted.
///
/// The C addition of the mantissa and exponent words is plain `uint32_t`
/// arithmetic, which wraps; `wrapping_add` reproduces that exactly. The result
/// is type-punned through a union in C, which is `f32::from_bits` here -- not a
/// numeric conversion.
#[inline]
fn half2float_impl(h: u16) -> f32 {
    let n = ((h >> 10) & 0x3f) as usize;
    let idx = (((h & 0x3ff) as usize) + (M__OFFSET[n] as usize)) & 0x7ff;
    let num = M__MANTISSA[idx].wrapping_add(M__EXPONENT[n]);
    f32::from_bits(num)
}

/// `float half2float(uint16_t h)`.
///
/// The parameter is declared as the full-width argument slot (`c_uint`) and
/// truncated here on purpose, because that is what the C function is observably
/// compiled to do:
///
/// * gcc's `half2float` opens with `movzwl`, i.e. the callee itself narrows the
///   incoming register to 16 bits. C therefore reads only bits 0..15 of the
///   argument slot and returns a defined value for *any* register contents.
/// * A Rust `extern "C"` `u16` parameter instead carries LLVM's `zeroext`
///   attribute, which says the caller already zero-extended. LLVM then shifts
///   the whole 32-bit register and, having proved `n < 64` from the `u16` type,
///   removes the `M__OFFSET`/`M__EXPONENT` bounds checks -- and it also folds
///   away an explicit `& 0x3f` written in the source. A caller that left junk in
///   the high half of the slot would thus read out of bounds, and the surviving
///   mantissa bounds check would abort, where C simply truncates and returns.
///
/// On every ABI of interest a `uint16_t` argument occupies a >= 32-bit slot
/// (`%edi` on x86-64 SysV, `w0` on AArch64), so taking `c_uint` here is the same
/// ABI footprint as `uint16_t`. For a conforming caller the mask is a no-op and
/// the result is identical; for a non-conforming one it matches C's truncation
/// bit-for-bit instead of reading out of bounds.
///
/// C body being translated:
/// ```c
/// float half2float(uint16_t h) {
///     union { float flt; uint32_t num; } out;
///     int n = h >> 10;
///     out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
///     return out.flt;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: c_uint) -> c_float {
    half2float_impl((h & 0xffff) as u16)
}
'''

OUT.write_text(
    HEADER
    + "\n"
    + emit("M__MANTISSA", "u32", mant, 6)
    + "\n\n"
    + emit("M__OFFSET", "u16", off, 8)
    + "\n\n"
    + emit("M__EXPONENT", "u32", exp, 6)
    + "\n"
    + BODY
)
print(f"wrote {OUT} ({len(mant)} mantissa, {len(off)} offset, {len(exp)} exponent)")
