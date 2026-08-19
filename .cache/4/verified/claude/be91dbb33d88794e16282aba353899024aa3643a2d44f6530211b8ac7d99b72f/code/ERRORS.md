# ERRORS.md — Error-surface table

Derived mechanically from the C source, not from documentation. The full text of
the only C function is:

```c
void sieve(int val) {
    while (1) {
        printf("%d\n", val);
        if (val % 10 == 9) {
            break;
        }
        val++;
    }
}
```

Mechanical grep results over `c_src/src/sieve.c` and `c_src/include/sieve.h`
(`return`, `assert`, `NULL`, `errno`, `exit`, `abort`, `-1`, `#if`, `MIN`, `MAX`,
comparisons):

* `return` statements: **0** (the function is `void` and falls out of the loop)
* `assert` / `abort` / `exit`: **0**
* `NULL` checks: **0** (the API takes no pointers)
* error enums / error codes / sentinel returns: **0**
* explicit range checks, min/max constants: **0**
* `#ifdef` / conditional compilation: **0** (only the header guard)
* branches in total: **1** — `if (val % 10 == 9) break;`
* **unchecked** library call: `printf`'s return value is discarded, so I/O
  failures are silently ignored
* **unchecked** arithmetic: `val++` has no overflow guard (signed overflow)

So this library has *no explicit rejection path at all*: it validates nothing and
cannot fail visibly. The error surface is therefore made of the implicit
rejections — the single loop-exit predicate, the ignored I/O error, the
unguarded overflow — plus the generic FFI boundaries. Each distinct one gets a
row, and each row gets a differential test in `tests/error_paths.rs`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `sieve` | no error return exists: any bounded input | returns normally (`void`), never reports failure; exit status of a child that calls it is 0 | `e1_no_error_return_path` | [x] |
| E2 | `sieve` | `val % 10 == 9` false for a **negative** `val` whose *floor*-mod would be 9, i.e. `val = -1` (`-1 % 10 == -1`, C truncates toward zero) | does **not** break; keeps incrementing up to and including `9`, then stops. A floor-mod translation would wrongly stop immediately | `e2_negative_remainder_never_equals_nine` | [x] |
| E3 | `sieve` | same trap for every negative residue class: `val ∈ {-9,-19,-29,-99,-109}` (`val % 10 == -9`) | never matches `9`; runs up to `9` | `e3_negative_residue_minus_nine` | [x] |
| E4 | `sieve` | `val` negative multiple of 10 (`val % 10 == 0`) e.g. `-10, -100, -1000` | no break until `9` | `e4_negative_multiple_of_ten` | [x] |
| E5 | `sieve` | `printf` fails: `stdout` redirected to `/dev/full` (every write returns `ENOSPC`) | return value ignored, no error propagated, loop still terminates on `val % 10 == 9`, child exits 0, zero bytes delivered | `e5_printf_write_error_ignored` | [x] |
| E6 | `sieve` | `printf` fails: file descriptor 1 **closed** before the call (`EBADF`) | return value ignored, function still returns normally, child exits 0 | `e6_stdout_closed` | [x] |
| E7 | `sieve` | signed-overflow boundary: `val ∈ [2147483640, 2147483647]` = `INT_MAX-7 .. INT_MAX`; no value `≥ val` ends in 9, so `val++` overflows `int` | undefined behaviour in ISO C; the shipped build (`gcc -O0`, no `-fwrapv`) wraps two's-complement to `INT_MIN` and keeps counting. Rust must reproduce the wrap and must **not** panic/abort | `e7_int_max_overflow_wraps` | [x] |
| E8 | `sieve` | `INT_MIN` (`-2147483648`), the most extreme accepted value | accepted, not rejected; prints `-2147483648` first and counts up (2147483658 lines) | `e8_int_min_accepted` | [x] |
| E9 | `sieve` | out-of-range value passed across the FFI boundary: caller passes a 64-bit argument whose high 32 bits are garbage (the C `int` parameter only occupies the low 32 bits — a C API accepts any bit pattern, incl. one with no valid "variant") | high bits ignored; behaves exactly as the truncated `int` value | `e9_ffi_high_bits_ignored` | [x] |
| E10 | `sieve` | generic-boundary sweep: zero, one-past-range values, and both ends of the domain (`-1, 0, 1, 8, 9, 10, INT_MIN, INT_MIN+1, INT_MAX-8 = 2147483639, INT_MAX`) | each is accepted (no validation) and produces the deterministic count-to-`…9` sequence, or the wrap for the overflow end | `e10_boundary_sweep` | [x] |
| E11 | `sieve` | null-pointer / length arguments | **N/A by construction** — the ABI is `void sieve(int)`: no pointer, buffer, length, or handle parameter exists to be null or oversized, so there is no null/length rejection to compare. Asserted structurally: the header declares exactly one `int` parameter | `e11_no_pointer_parameters` (documents + asserts the one-arg ABI via symbol lookup) | [x] |

## Verification evidence

All 11 rows have a passing differential test in `tests/error_paths.rs`, run
against both `.so` files loaded with `libloading`:

```
running 11 tests
test e1_no_error_return_path ... ok
test e2_negative_remainder_never_equals_nine ... ok
test e3_negative_residue_minus_nine ... ok
test e4_negative_multiple_of_ten ... ok
test e5_printf_write_error_ignored ... ok
test e6_stdout_closed ... ok
test e7_int_max_overflow_wraps ... ok
test e8_int_min_accepted ... ok
test e9_ffi_high_bits_ignored ... ok
test e10_boundary_sweep ... ok
test e11_no_pointer_parameters ... ok
test result: ok. 11 passed; 0 failed
```

Notes on the two rows that involve C undefined/unspecified behaviour:

* **E7 (signed overflow).** The comparison is against the shipped C build, and the
  behaviour was additionally checked to be stable across C optimisation levels:
  `gcc -O0` (the default cmake configuration used here) and `gcc -O2` both wrap
  `2147483647 -> -2147483648` and keep counting, which is exactly what the Rust
  `wrapping_add` reproduces. Because such a run emits ~2^31 lines, C and Rust are
  compared on an 8 KiB output prefix taken from a forked child, and the wrap
  itself is asserted to appear in both streams.
* **E5/E6 (ignored `printf` failure).** Compared by raw `waitpid` status of a
  forked child, so "returns normally" is checked as an observable fact (exit 0,
  no signal, no abort) rather than inferred. Rust must not panic across the
  `extern "C"` boundary here; a panic would abort and change the status.

Rows are exercised in both the dev and the release profile (the release profile
sets `panic = "abort"`), and under the single feature combination that exists
(`Cargo.toml` declares no `[features]`).

Sensitivity of these tests was validated by mutation (`./mutation_check.sh`): a
floor-mod translation, a saturating increment, an off-by-one terminator and seven
other deliberate defects were each detected — see `CONFIGS.md` for the table.
