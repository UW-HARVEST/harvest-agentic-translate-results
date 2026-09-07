#!/usr/bin/env python3
"""Harness sanity check: inject a small behavioural change into the Rust
translation and confirm the differential suite catches it.

Proves the tests are not vacuous. Always restores .backup/lib.rs.good.
"""
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "src" / "lib.rs"
GOOD = ROOT / ".backup" / "lib.rs.good"

MUTANTS = [
    ("M1 convert_pix alpha 0xFF -> 0xFE",
     "cp_pixel_t { r, g, b, a: 0xFF }", "cp_pixel_t { r, g, b, a: 0xFE }"),
    ("M2 cp_rev16 nibble mask typo",
     "a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);",
     "a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0E) << 4);"),
    ("M3 error message text changed",
     "Detected unknown block type within input stream.",
     "Detected unknown block type within input stream!"),
    ("M4 literal out-of-space check off by one",
     "if !((*s).out.offset(1) <= (*s).out_end) {",
     "if !((*s).out.offset(2) <= (*s).out_end) {"),
    ("M5 stored-block length comparison <= -> <",
     "if !((*s).bits_left / 8 <= len_val as c_int) {",
     "if !((*s).bits_left / 8 < len_val as c_int) {"),
    ("M6 cp_build canonical code shift",
     "codes[n] = (codes[n - 1].wrapping_add(counts[n - 1])) << 1;",
     "codes[n] = (codes[n - 1].wrapping_add(counts[n - 1])) << 2;"),
    ("M7 dist==1 memset path replaced by generic copy",
     "                1 => {\n                    ptr::write_bytes(dst as *mut u8, ptr::read(src as *const u8), length as usize);\n                }\n",
     "                1 => {\n                    ptr::write_bytes(dst as *mut u8, 0u8, length as usize);\n                }\n"),
    ("M8 cp_paeth tie-break flipped",
     "if pa <= pb && pa <= pc {", "if pa < pb && pa <= pc {"),
    ("M9 dropped an exported global's no_mangle",
     "#[unsafe(no_mangle)]\npub static mut cp_len_base",
     "pub static mut cp_len_base"),
]

# Mutants that CANNOT be detected through the ABI, with the reason. `cp_paeth` is
# only called from `cp_unfilter`, which is `static` in lib.c and has zero call
# sites (verified: `grep -n cp_unfilter c_src/src/lib.c` shows only its
# definition). The whole PNG filter/chunk cluster is therefore dead in both
# libraries, so no differential test can observe a change to it.
EXPECTED_UNDETECTABLE = {
    "M8 cp_paeth tie-break flipped":
        "cp_paeth is reachable only from cp_unfilter, a static fn with no callers",
}

TESTS = ["smoke", "phase_b_valid", "phase_c_errors"]


def run_suite():
    """Return (n_failed, n_passed) across TESTS, or (-1, -1) if it won't build."""
    b = subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT,
                       capture_output=True, text=True)
    if b.returncode != 0:
        return -1, -1
    args = ["cargo", "test", "--release"]
    for t in TESTS:
        args += ["--test", t]
    args += ["--", "--test-threads=1"]
    r = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, timeout=600)
    text = r.stdout + r.stderr
    failed = passed = 0
    for m in re.finditer(r"(\d+) passed; (\d+) failed", text):
        passed += int(m.group(1))
        failed += int(m.group(2))
    # A mutant that makes a whole test binary abort shows up as a signal, not a
    # per-test count; treat that as "detected" too.
    if "SIGABRT" in text or "signal:" in text:
        failed = max(failed, 1)
    return failed, passed


def main():
    # Take our own pristine snapshot inside the crate (never /tmp, which is
    # shared and can be clobbered by other processes on the machine).
    GOOD.parent.mkdir(parents=True, exist_ok=True)
    if not GOOD.exists():
        shutil.copy(SRC, GOOD)
    shutil.copy(GOOD, SRC)
    base_failed, base_passed = run_suite()
    print(f"baseline (pristine):        failed={base_failed} passed={base_passed}")
    if base_failed != 0 or base_passed == 0:
        shutil.copy(GOOD, SRC)
        sys.exit("baseline is not green; aborting mutation check")

    undetected = []
    for name, old, new in MUTANTS:
        shutil.copy(GOOD, SRC)
        s = SRC.read_text()
        if old not in s:
            shutil.copy(GOOD, SRC)
            sys.exit(f"mutation pattern not found for {name!r}: {old!r}")
        SRC.write_text(s.replace(old, new, 1))
        failed, passed = run_suite()
        verdict = "DETECTED" if failed != 0 else "NOT DETECTED"
        if failed == -1:
            verdict = "DETECTED (does not compile)"
        if failed == 0 and name in EXPECTED_UNDETECTABLE:
            verdict = f"UNDETECTABLE BY DESIGN ({EXPECTED_UNDETECTABLE[name]})"
        print(f"{name:48} failed={failed:<5} {verdict}")
        if failed == 0 and name not in EXPECTED_UNDETECTABLE:
            undetected.append(name)

    shutil.copy(GOOD, SRC)
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, check=True)
    f, p = run_suite()
    print(f"restored:                   failed={f} passed={p}")
    assert f == 0 and p == base_passed, "restore did not return to the baseline"
    if undetected:
        sys.exit(f"suite failed to detect: {undetected}")
    print(
        f"\n{len(MUTANTS) - len(EXPECTED_UNDETECTABLE)} of {len(MUTANTS)} mutants detected; "
        f"{len(EXPECTED_UNDETECTABLE)} undetectable by design. Suite is not vacuous."
    )


if __name__ == "__main__":
    main()
