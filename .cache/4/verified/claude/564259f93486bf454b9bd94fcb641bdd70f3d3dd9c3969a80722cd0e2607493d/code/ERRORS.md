# ERRORS.md — Error-surface table

Derived **mechanically** from the C source, not from docs or assumptions.

## Mechanical derivation

```sh
grep -nE 'RETURN_ERROR|return *-|return *NULL|assert|errno|_MAX|_MIN|if *\(|switch|while|for|\?|else|goto|exit|abort|<=|>=|==|!=|&&|\|\|' \
     c_src/src/driver.c c_src/include/driver.h
#   -> NO MATCHES
```

Result: the C library contains

* **0** error-return macros (`RETURN_ERROR`, …)
* **0** `return -1` / `return NULL` / error enums (the only function returns `void`)
* **0** `assert`s
* **0** explicit range checks, null checks, or `if`/`switch`/loop branches
* **0** `MIN`/`MAX` constants
* **0** `errno` uses
* **0** enum types anywhere in the public header

`void driver(const char *s1, const char *s2)` returns nothing and validates
nothing. Therefore **there is no in-band error/rejection surface to match**: the
function cannot report an error.

Consequently the rows below are the *generic C-API boundary conditions* the task
requires us to cover anyway. For this API the only observable "rejection" is the
C runtime's own fault behaviour on invalid pointers, plus the degenerate-but-
**valid** empty-string inputs. Each row is verified differentially: the same
condition is fed to the C `.so` and the Rust `.so` and the observable result
(process exit status / terminating signal, or captured stdout bytes) must match.

Pointer rows are undefined behaviour in C. We do not claim the C standard
defines them; we assert only that **the Rust build reproduces the C build's
actual observed behaviour on this platform**, which is what a caller sees. Each
pointer row runs in a `fork()`ed child so a fault is observable instead of
killing the test runner, and C and Rust children are compared on
`WIFSIGNALED`/`WTERMSIG`/`WEXITSTATUS`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `driver` | `s1 == NULL`, `s2` valid non-empty | no in-band error; glibc `strcspn` dereferences `s1` → child dies on `SIGSEGV` (11), nothing printed | `err_e1_s1_null` | [x] |
| E2 | `driver` | `s1 == NULL`, `s2 == ""` (empty reject set) | dereferences `s1` → child dies on `SIGSEGV` (11) | `err_e2_s1_null_s2_empty` | [x] |
| E3 | `driver` | `s1` valid non-empty, `s2 == NULL` | dereferences `s2` → child dies on `SIGSEGV` (11) | `err_e3_s2_null` | [x] |
| E4 | `driver` | `s1 == ""` (empty), `s2 == NULL` | glibc reads `s2` before/independently of finishing `s1` → child dies on `SIGSEGV` (11) | `err_e4_s1_empty_s2_null` | [x] |
| E5 | `driver` | `s1 == NULL` **and** `s2 == NULL` | child dies on `SIGSEGV` (11) | `err_e5_both_null` | [x] |
| E6 | `driver` | `s1 = (char*)1` — misaligned, unmapped, non-null garbage pointer | child dies on `SIGSEGV` (11) | `err_e6_s1_garbage_ptr` | [x] |
| E7 | `driver` | `s2 = (char*)1` — misaligned, unmapped, non-null garbage pointer | child dies on `SIGSEGV` (11) | `err_e7_s2_garbage_ptr` | [x] |
| E8 | `driver` | `s1` **unterminated**: buffer ends exactly at an unmapped page boundary, no NUL, and no byte of `s1` occurs in `s2` (scan must run off the end) | child dies on `SIGSEGV` (11) | `err_e8_s1_unterminated_page_edge` | [x] |
| E9 | `driver` | `s2` **unterminated**: reject buffer ends exactly at an unmapped page boundary, no NUL | child dies on `SIGSEGV` (11) | `err_e9_s2_unterminated_page_edge` | [x] |
| E10 | `driver` | zero-length input: `s1 == ""`, `s2 == ""` (both empty — the "zero length" boundary) | **not** an error: prints `0\n` | `err_e10_zero_length_both_empty` | [x] |
| E11 | `driver` | zero-length reject set with non-empty `s1`: `s2 == ""` | **not** an error: prints `strlen(s1)\n` (no byte can be rejected) | `err_e11_zero_length_reject_set` | [x] |
| E12 | `driver` | "oversized" length: `s1` is 1 MiB with no rejected byte, forcing the maximal in-range return value | **not** an error: prints `1048576\n`; exercises multi-page scan and wide `%zu` formatting | `err_e12_oversized_length` | [x] |
| E13 | `driver` | one step past the byte range: `s2` contains **all 255** non-NUL byte values `0x01..0xFF`, so every possible non-NUL `s1` byte is rejected | **not** an error: prints `0\n` for any non-empty `s1`; proves the reject table covers the full `unsigned char` domain with no off-by-one at `0xFF` | `err_e13_full_byte_domain_reject` | [x] |
| E14 | `driver` | high-bit bytes (`0x80..0xFF`) — the values where `char` is **negative** on this platform, i.e. the sign-extension trap when indexing a reject table | **not** an error: must be treated as `unsigned char`; no panic, no out-of-bounds index | `err_e14_high_bit_sign_extension` | [x] |
| E15 | `driver` | embedded NUL in the middle of both buffers (bytes after the NUL are unreachable) | **not** an error: scan/reject set both stop at the first NUL; trailing bytes ignored | `err_e15_embedded_nul_terminates` | [x] |

## Finding: the `debug` profile changes the fault signal (rows E1–E5)

The one divergence this phase uncovered, and its resolution:

* Against the **release** `cdylib` (`target/release/libdriver.so` — the
  deliverable, built with the crate's declared `panic = "abort"`), Rust and C die
  with **exactly the same signal, `SIGSEGV` (11)**, on every one of E1–E5 and on
  the full generic invalid-pointer matrix. All 16 rows pass.
* Against the **debug** `cdylib`, the null-pointer rows E1–E5 instead died with
  `SIGABRT` (6) while C died with `SIGSEGV` (11).

Diagnosis: this is *not* a translation defect. Since Rust 1.78, rustc emits
optional "UB checks" (null and alignment checks on raw-pointer dereferences)
gated on `debug_assertions`; they convert a would-be fault into a controlled Rust
panic. Proof: rebuilding the **unoptimised** `dev` profile with
`RUSTFLAGS="-C debug-assertions=off"` restores `SIGSEGV` parity, so the change
comes from the assertion flag and not from optimisation.

The non-null garbage-pointer rows (E6, E7) and the unterminated-buffer rows
(E8, E9) pass in *both* profiles, because those addresses are non-null and
byte-aligned, so no UB check fires and a genuine `SIGSEGV` results.

The harness encodes this explicitly rather than hiding it:
`Harness::assert_fault_parity` demands an identical signal by default, and only
tolerates `SIGABRT` on the Rust side when the runner sets
`DRIVER_RUST_UB_CHECKS=1` — which `run_all_tests.sh` does solely for the extra
debug-artifact pass. Both profiles are exercised on every run.

### Notes on rows that are deliberately absent

* **Out-of-range enum values across FFI** — not applicable: `driver.h` declares
  no enum, and the function takes no integer/flag parameter, so there is no
  `int`-typed variant space an out-of-range value could occupy. E13/E14 are the
  analogous "value with no valid meaning" checks for this API's actual domain
  (the full `unsigned char` byte range, including the sign-extension boundary).
* **Length / size arguments** — not applicable: the API takes no length. The
  zero/oversized-length boundary is instead expressed through string length
  (E10, E11, E12).
* **Return-code mismatches** — not applicable: return type is `void`. All
  observable output is stdout bytes (compared byte-for-byte) or the process
  termination status (compared exactly).
