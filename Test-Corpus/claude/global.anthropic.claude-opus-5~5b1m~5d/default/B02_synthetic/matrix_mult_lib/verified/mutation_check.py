#!/usr/bin/env python3
"""Harness self-validation: does the differential suite actually catch bugs?

Each entry below injects one deliberate divergence into the Rust translation,
rebuilds the release cdylib, runs the named tests, and expects them to FAIL.
A mutation that is NOT caught means the differential suite has a blind spot.

    python3 mutation_check.py <name>

Results — all 10 mutations are caught:

    write.rs EINVAL->21           err_13 / err_17 / g1
    to_string separator           20 Phase-B rows
    init column message 0-based   err_07 / cfg_09 / cfg_13
    driver EXIT_FAILURE->2        err_18 / err_19
    allocate clamp negative width err_03 / err_08 / g4 / g5
    buffer_size saturating        err_12
    multiply saturating           cfg_24 / cfg_34
    free_matrix rows NULL guard   err_24b
    write_to_file NULL filename   err_17
    "insufficient rows" wording   err_06

The mutated source file is always restored, even on failure.
"""
import subprocess, shutil, sys

# name -> (file, original snippet, mutated snippet, tests that must fail)
MUTATIONS = {
 "write_einval": ("src/write.rs",
    "return EINVAL;",
    "return 21;",
    ["err_13_write_null_content", "err_17_write_null_filename", "g1_null_pointers"]),
 "to_string_separator": ("src/matrix.rs",
    "if j < width.wrapping_sub(1) {",
    "if j < width {",
    ["cfg_06_init_exact_fit", "cfg_17_to_string_random", "cfg_25_pipeline_random"]),
 "init_col_msg_0based": ("src/matrix.rs",
    "i.wrapping_add(1),\n                );",
    "i,\n                );",
    ["err_07_insufficient_columns", "cfg_09_init_collapsed_delimiters"]),
 "driver_exit_code": ("src/driver.rs",
    "return EXIT_FAILURE;\n    }\n    let mat_b",
    "return 2;\n    }\n    let mat_b",
    ["err_18_driver_mat_a_fails"]),
 "alloc_clamp_neg_width": ("src/matrix.rs",
    "let col_bytes = (width as isize as usize)",
    "let col_bytes = (width.max(0) as isize as usize)",
    ["err_03_allocate_negative_width", "err_08_init_returns_null_from_failed_alloc",
     "g4_one_past_range"]),
 "bufsize_sat": ("src/matrix.rs",
    "let buffer_size: c_int = height\n        .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))\n        .wrapping_add(height)\n        .wrapping_add(1);",
    "let buffer_size: c_int = height\n        .saturating_mul(width.saturating_mul(10).saturating_add(width))\n        .saturating_add(height)\n        .saturating_add(1);",
    ["err_12_to_string_buffer_size_overflow"]),
 "mul_sat": ("src/matrix.rs",
    "*out = (*out).wrapping_add(a.wrapping_mul(b));",
    "*out = (*out).saturating_add(a.saturating_mul(b));",
    ["cfg_24_mul_overflow", "cfg_34_driver_1x1_overflow"]),
 "free_guard": ("src/matrix.rs",
    "    if mat.is_null() {\n        return;\n    }\n\n    let mut i: c_int = 0;",
    "    if mat.is_null() || (*mat).matrix.is_null() {\n        return;\n    }\n\n    let mut i: c_int = 0;",
    ["err_24_free_matrix_null_rows_nonpositive_height",
     "err_24b_free_matrix_null_rows_positive_height"]),
 "null_fname_guard": ("src/write.rs",
    "    let file = fopen(filename, c\"w\".as_ptr());",
    "    if filename.is_null() { return EINVAL; }\n    let file = fopen(filename, c\"w\".as_ptr());",
    ["err_17_write_null_filename"]),
 "rows_msg": ("src/matrix.rs",
    'c"Insufficient rows in input string.\\n".as_ptr()',
    'c"Insufficient rows in input.\\n".as_ptr()',
    ["err_06_insufficient_rows"]),
}


def rebuild():
    subprocess.run(["bash", "-c", "touch src/*.rs"], check=False)
    return subprocess.run(["cargo", "build", "--release", "--offline"],
                          capture_output=True, text=True, timeout=300)


def run(key):
    f, old, new, tests = MUTATIONS[key]
    src = open(f).read()
    if old not in src:
        print(f"ERROR {key}: pattern not found in {f}")
        return False
    shutil.copy(f, f + ".bak")
    caught = False
    try:
        open(f, "w").write(src.replace(old, new, 1))
        b = rebuild()
        if b.returncode != 0:
            print(f"BUILD FAILED {key}: {b.stderr[-500:]}")
            return False
        for t in tests:
            try:
                r = subprocess.run(
                    ["cargo", "test", "--release", "--offline", "--",
                     "--exact", t, "--test-threads=1"],
                    capture_output=True, text=True, timeout=120)
                ok = r.returncode == 0
                note = "passed (not caught)" if ok else "FAILED -> caught"
            except subprocess.TimeoutExpired:
                ok, note = False, "TIMEOUT (divergence hangs) -> caught"
            print(f"  {t}: {note}")
            caught = caught or not ok
        print(("caught      " if caught else "NOT CAUGHT  ") + key)
    finally:
        shutil.move(f + ".bak", f)
        rebuild()
    return caught


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--list":
        print(" ".join(MUTATIONS))
        sys.exit(0)
    keys = sys.argv[1:] or list(MUTATIONS)
    bad = [k for k in keys if not run(k)]
    if bad:
        print("NOT CAUGHT:", " ".join(bad))
    sys.exit(1 if bad else 0)
