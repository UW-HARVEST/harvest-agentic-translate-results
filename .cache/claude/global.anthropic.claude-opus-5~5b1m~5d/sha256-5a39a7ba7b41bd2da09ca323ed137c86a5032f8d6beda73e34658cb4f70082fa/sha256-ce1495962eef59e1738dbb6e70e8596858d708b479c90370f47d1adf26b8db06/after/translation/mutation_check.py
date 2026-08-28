#!/usr/bin/env python3
"""Mutation-test the differential suite.

Injects deliberate bugs into translation/src/lib.rs, rebuilds the Rust .so, and
verifies the test suite CATCHES each one. A mutation that survives means the
suite has a blind spot. The original source is always restored.
"""
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "src" / "lib.rs"
BAK = ROOT / "src" / "lib.rs.mutation_backup"


def table_span(text, marker):
    i = text.index(marker) + len(marker)
    j = text.index("];", i)
    return i, j


def set_table_entry(text, marker, index, new_literal):
    """Replace the `index`-th hex literal inside the table starting at `marker`."""
    i, j = table_span(text, marker)
    block = text[i:j]
    hits = list(re.finditer(r"0x[0-9a-fA-F]+", block))
    assert len(hits) == 512, f"expected 512 entries, found {len(hits)}"
    h = hits[index]
    old = h.group(0)
    assert old != new_literal, f"mutation is a no-op at index {index}"
    newblock = block[: h.start()] + new_literal + block[h.end() :]
    return text[:i] + newblock + text[j:], old


def mut_base_entry(text, index, new):
    return set_table_entry(text, "M_BASE: [u16; 512] = [", index, new)[0]


def mut_shift_entry(text, index, new):
    return set_table_entry(text, "M_SHIFT: [u8; 512] = [", index, new)[0]


def replace_once(text, old, new):
    assert text.count(old) == 1, f"expected exactly 1 occurrence of {old!r}, got {text.count(old)}"
    return text.replace(old, new)


MUTATIONS = {
    # Drop the sign bit from the table index: classic masking off-by-one.
    "M1_index_mask_drops_sign": lambda t: replace_once(
        t, "let j: u32 = (n >> 23) & 0x1ff;", "let j: u32 = (n >> 23) & 0x0ff;"
    ),
    # The "obvious NaN fix": make exponent 255 discard the mantissa like the
    # saturating region does. This is the highest-risk blind spot.
    "M2_shift255_13_to_24": lambda t: mut_shift_entry(t, 255, "0x18"),
    # Same, for the negative NaN index.
    "M3_shift511_13_to_24": lambda t: mut_shift_entry(t, 511, "0x18"),
    # Single-entry corruption deep in the negative half of the base table.
    "M4_base300_off_by_one": lambda t: mut_base_entry(t, 300, "0x8001"),
    # Single-entry corruption in the varying-shift subnormal region.
    "M5_shift103_23_to_22": lambda t: mut_shift_entry(t, 103, "0x16"),
    # Mantissa mask off-by-one.
    "M6_mantissa_mask": lambda t: replace_once(
        t, "let mantissa = (n & 0x007f_ffff) >> shift;", "let mantissa = (n & 0x00ff_ffff) >> shift;"
    ),
    # A plausible "improvement": round-to-nearest instead of C's truncation.
    "M7_round_to_nearest": lambda t: replace_once(
        t,
        "let mantissa = (n & 0x007f_ffff) >> shift;",
        "let mantissa = ((n & 0x007f_ffff) + (1u32 << shift >> 1)) >> shift;",
    ),
    # A plausible "improvement": preserve NaN-ness instead of letting small
    # payloads degenerate to Infinity.
    "M8_preserve_nan": lambda t: replace_once(
        t,
        "    let n: u32 = f32::to_bits(flt);",
        "    if flt.is_nan() { return 0x7e00; }\n    let n: u32 = f32::to_bits(flt);",
    ),
    # Boundary shift of a whole region: first saturating exponent.
    "M9_base143_not_inf": lambda t: mut_base_entry(t, 143, "0x7bff"),
    # Sign of zero: negative underflow yields +0 instead of -0.
    "M10_base256_neg_zero": lambda t: mut_base_entry(t, 256, "0x0000"),
}

TEST_BINS = ["phase_b_valid", "phase_c_errors", "phase_d_exhaustive"]


def run_suite():
    """Return (ok, per_test_failures) for the release suite."""
    b = subprocess.run(
        ["cargo", "build", "--release"], cwd=ROOT, capture_output=True, text=True
    )
    if b.returncode != 0:
        return False, ["BUILD FAILED: " + b.stderr[-400:]]

    failures = []
    for tb in TEST_BINS:
        env = {"EXHAUSTIVE_STRIDE": "1021"} if tb == "phase_d_exhaustive" else {}
        import os

        e = dict(os.environ, **env)
        r = subprocess.run(
            ["cargo", "test", "--release", "--test", tb, "--", "--test-threads=8"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=e,
        )
        for line in r.stdout.splitlines():
            if line.startswith("test ") and (" FAILED" in line or line.endswith("FAILED")):
                failures.append(line.split()[1])
    return len(failures) == 0, failures


def main():
    shutil.copy(SRC, BAK)
    original = BAK.read_text()
    results = {}
    try:
        # Baseline must be green.
        ok, fails = run_suite()
        if not ok:
            print(f"BASELINE IS NOT GREEN: {fails}", file=sys.stderr)
            return 2
        print("baseline: all tests pass (unmutated)\n")

        for name, fn in MUTATIONS.items():
            SRC.write_text(fn(original))
            ok, fails = run_suite()
            caught = not ok
            results[name] = (caught, fails)
            status = "CAUGHT" if caught else "*** SURVIVED (blind spot!) ***"
            print(f"{name:32s} {status}")
            if caught:
                shown = sorted(set(fails))
                print(f"{'':32s}   caught by {len(shown)} test(s): {', '.join(shown[:6])}"
                      + (" ..." if len(shown) > 6 else ""))
    finally:
        shutil.copy(BAK, SRC)
        BAK.unlink()
        subprocess.run(["cargo", "build", "--release"], cwd=ROOT, capture_output=True)
        print("\n[restored original src/lib.rs]")

    survived = [n for n, (c, _) in results.items() if not c]
    print(f"\n{len(results) - len(survived)}/{len(results)} mutations caught")
    if survived:
        print("SURVIVORS (test-suite blind spots): " + ", ".join(survived))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
