#!/usr/bin/env python3
"""Mutation test: prove the differential harness is not vacuous.

Each mutation injects one class of bug into translation/src/lib.rs, rebuilds both
cdylibs, and records which test files catch it. A mutation that is NOT caught is
a hole in the test suite.
"""
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(ROOT, "translation")
LIB = os.path.join(CRATE, "src", "lib.rs")
ORIG = os.path.join(ROOT, "lib.rs.orig")

TESTS = ["phase_b_valid", "phase_c_errors", "phase_d_symbols"]

# (name, old, new) — `old` must occur in the original source.
MUTATIONS = [
    # dispatch
    ("dispatch-swap-0-1",
     "const CB_PROTANOPIA: c_int = 0;\nconst CB_DEUTERANOPIA: c_int = 1;",
     "const CB_PROTANOPIA: c_int = 1;\nconst CB_DEUTERANOPIA: c_int = 0;"),
    ("dispatch-add-default",
     "        _ => {}",
     "        _ => unsafe { protanopia(r, g, b) },"),
    ("dispatch-clamp-out-of-range",
     "        _ => {}",
     "        _ => unsafe { tritanopia(r, g, b) },"),
    ("dispatch-mod3-normalise",
     "    match impairment {",
     "    let impairment = impairment.rem_euclid(3);\n    match impairment {"),

    # coefficients (1-ULP class errors)
    ("coeff-1ulp-P_RR", "const P_RR: f32 = 0.17055699213417;",
     "const P_RR: f32 = 0.17055702;"),
    ("coeff-1ulp-P_GG", "const P_GG: f32 = 0.82944300785005;",
     "const P_GG: f32 = 0.8294431;"),
    ("coeff-sign-T_GR", "const T_GR: f32 = -4.486E-11;",
     "const T_GR: f32 = 4.486E-11;"),
    ("coeff-round-T_RB", "const T_RB: f32 = 0.12739886341072;",
     "const T_RB: f32 = 0.1273989;"),
    ("coeff-drop-tiny-P_RB", "const P_RB: f32 = 2.91188E-9;",
     "const P_RB: f32 = 0.0;"),

    # operators / associativity
    ("op-sub-to-add", "        subss(t3, t4)", "        addss(t3, t4)"),
    ("op-reassociate-add", "        addss(t4, t3)", "        addss(t3, t4)"),

    # SSE NaN semantics
    ("nan-drop-sse-emulation",
     "fn sse_scalar(dst: f32, src: f32, computed: f32) -> f32 {",
     "fn sse_scalar(_d: f32, _s: f32, computed: f32) -> f32 { computed }\n"
     "#[allow(dead_code)]\nfn sse_scalar_dead(dst: f32, src: f32, computed: f32) -> f32 {"),
    ("nan-src-before-dst",
     "    if dst.is_nan() {\n        quiet_nan(dst)\n    } else if src.is_nan() {\n        quiet_nan(src)",
     "    if src.is_nan() {\n        quiet_nan(src)\n    } else if dst.is_nan() {\n        quiet_nan(dst)"),
    ("nan-quiet-noop",
     "    f32::from_bits(x.to_bits() | 0x0040_0000)", "    x"),
    ("nan-quiet-drops-payload",
     "    f32::from_bits(x.to_bits() | 0x0040_0000)",
     "    f32::from_bits((x.to_bits() & 0x8000_0000) | 0x7FC0_0000)"),
    ("nan-default-qnan-sign",
     "const X86_DEFAULT_QNAN: u32 = 0xFFC0_0000;",
     "const X86_DEFAULT_QNAN: u32 = 0x7FC0_0000;"),

    # aliasing: reversing the write order is observable only when two of the
    # three pointers alias (rows A2..A5), because the last write wins.
    ("alias-reverse-write-order",
     "    unsafe {\n        *red = out_r;\n        *green = out_g;\n        *blue = out_b;\n    }",
     "    unsafe {\n        *blue = out_b;\n        *green = out_g;\n        *red = out_r;\n    }"),

    # symbol surface
    ("symbol-drop-no-mangle", "#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn colourblind",
     "#[allow(dead_code)]\npub unsafe extern \"C\" fn colourblind"),
    ("symbol-export-static-helper", "unsafe fn protanopia(",
     "#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn Protanopia(red: *mut f32, green: *mut f32, blue: *mut f32)"
     " { unsafe { protanopia(red, green, blue) } }\nunsafe fn protanopia("),
]


def sh(cmd, **kw):
    return subprocess.run(cmd, cwd=CRATE, stdout=subprocess.DEVNULL,
                          stderr=subprocess.DEVNULL, **kw).returncode


def restore():
    shutil.copyfile(ORIG, LIB)


def build():
    return sh(["cargo", "build", "--release"]) == 0 and sh(["cargo", "build"]) == 0


def main():
    if not os.path.exists(ORIG):
        shutil.copyfile(LIB, ORIG)
    src = open(ORIG).read()

    restore()
    if not build():
        print("baseline build FAILED", file=sys.stderr)
        return 1
    baseline_fail = [t for t in TESTS
                     if sh(["cargo", "test", "--test", t], timeout=400) != 0]
    if baseline_fail:
        print(f"baseline tests already failing: {baseline_fail}", file=sys.stderr)
        return 1
    print("baseline: all tests pass\n")

    uncaught = []
    for name, old, new in MUTATIONS:
        if src.count(old) < 1:
            print(f"{name:<32} SKIPPED (pattern not found)")
            continue
        n = src.count(old)
        open(LIB, "w").write(src.replace(old, new))
        if not build():
            print(f"{name:<32} BUILD FAILED (mutation invalid)")
            restore()
            continue
        caught = []
        for t in TESTS:
            if sh(["cargo", "test", "--test", t], timeout=400) != 0:
                caught.append(t.replace("phase_", "").replace("_valid", "")
                              .replace("_errors", "").replace("_symbols", ""))
        if caught:
            print(f"{name:<32} caught by: {','.join(caught)}   (x{n} sites)")
        else:
            print(f"{name:<32} *** NOT CAUGHT ***   (x{n} sites)")
            uncaught.append(name)
        restore()

    restore()
    build()
    os.remove(ORIG)
    print()
    if uncaught:
        print(f"HOLES IN THE SUITE: {uncaught}")
        return 1
    print("every mutation was caught; the harness is not vacuous")
    return 0


if __name__ == "__main__":
    sys.exit(main())
