# ERRORS.md — Phase A: Error-surface table

## How this table was derived (mechanical grep of `c_src/`)

```
$ grep -nE '\breturn\b'                            src/*.c include/*.h   -> (no matches)
$ grep -nE 'assert'                                src/*.c include/*.h   -> (no matches)
$ grep -nE 'NULL|nullptr|!= *0|== *0'              src/*.c include/*.h   -> (no matches)
$ grep -nEi 'error|errno|fail|invalid|-1|enum|MIN|MAX' src/*.c include/*.h
      include/driver.h:25:#define DRIVER_H_        <- include guard only
$ grep -nE '\b(if|switch|else|while|for)\b|#if'    src/*.c include/*.h
      include/driver.h:24:#ifndef DRIVER_H_        <- include guard only
```

The complete C implementation is:

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}
```

**Findings:** the C library contains

* **0** error-return macros / `return` statements (the function is `void`),
* **0** `assert`s,
* **0** explicit range checks, null checks, or min/max constants,
* **0** error enums or error codes,
* **0** `if` / `switch` / `#ifdef` branches (the only preprocessor conditional is the
  `DRIVER_H_` include guard).

There is therefore **no explicit rejection path at all**: `driver` accepts every
argument pair and returns `void`. It cannot report an error to its caller.

Consequently the error surface is entirely **implicit** — the hard-failure conditions
inherited from the two functions it calls (`strcspn`, `printf`) plus the contract of
`const char *` meaning "pointer to a NUL-terminated string". Those are enumerated
below. Because the failures are process-fatal signals rather than return values, the
differential tests in `tests/error_paths.rs` run each call in a **forked child** and
compare the child's *exact* termination status (normal exit vs. terminating signal
number) and stdout bytes between C and Rust — not merely "both failed somehow".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `driver` | `s1 == NULL`, `s2` a valid string | `strcspn` dereferences a null pointer → child killed by `SIGSEGV` (11); nothing written to stdout | `err_01_s1_null` |
| 2 | `driver` | `s1 == NULL`, `s2 == ""` (valid, empty) | `strcspn` dereferences a null pointer → child killed by `SIGSEGV` (11); nothing written to stdout | `err_02_s1_null_s2_empty` |
| 3 | `driver` | `s1` a valid non-empty string, `s2 == NULL` | reject set is read from a null pointer → child killed by `SIGSEGV` (11); nothing written to stdout | `err_03_s2_null` |
| 4 | `driver` | `s1 == ""` (valid, empty), `s2 == NULL` | **`SIGSEGV` (11)**, nothing printed. Determined empirically, *not* assumed: the C library's `strcspn` builds its reject-set table before looking at `s1`, so `s2` is dereferenced unconditionally and an empty `s1` does **not** short-circuit. This row caught a real translation bug — see "Divergence found" below. | `err_04_s2_null_s1_empty` |
| 5 | `driver` | `s1 == NULL` **and** `s2 == NULL` | `strcspn` dereferences a null pointer → child killed by `SIGSEGV` (11); nothing written to stdout | `err_05_both_null` |
| 6 | `driver` | `s1` points at a buffer with **no NUL terminator**, terminated only by an unmapped guard page; `s2` contains no byte present in the buffer | scan runs off the end of the mapping → child killed by `SIGSEGV` (11) | `err_06_s1_unterminated` |
| 7 | `driver` | `s2` points at a buffer with **no NUL terminator** (unmapped guard page after it); `s1` non-empty and containing no byte present in `s2` | reject-set scan runs off the end of the mapping → child killed by `SIGSEGV` (11) | `err_07_s2_unterminated` |
| 7b | `driver` | `s2` **unterminated** (as row 7) but `s1 == ""` — the result is knowable without reading `s2` at all | still **`SIGSEGV` (11)**: the reject set is scanned unconditionally, at every length (verified for 1, 2, 3 and 4096 bytes). Same root cause as row 4. | `err_07b_s2_unterminated_empty_s1` |
| 7c | `driver` | `s2` **properly terminated** but flush against an unmapped guard page, `s1 == ""` — reject lengths 0..7 | **no fault**, prints `0\n`. The mirror of 7b: it pins the read *extent*, so the fix for 7b cannot be "read `s2[0]` and `s2[1]` unconditionally" | `err_07c_s2_read_extent_exact` |
| 8 | `driver` | `s1` misaligned / at the very end of a page, valid and NUL-terminated (`strcspn` SIMD over-read boundary; not an error but the classic false-positive fault case) | **no fault**: prints the correct count. Guards against a Rust or C implementation that over-reads past the NUL across a page boundary | `err_08_page_boundary_s1` |
| 9 | `driver` | `s2` NUL-terminated but placed flush against the end of a page (reject-set over-read boundary) | **no fault**: prints the correct count | `err_09_page_boundary_s2` |
| 10 | `driver` | `s1` = 1-byte string `"\0"` at the last byte of a page whose successor page is unmapped, `s2` valid | **no fault**: prints `0\n` | `err_10_page_boundary_empty_s1` |
| 11 | `driver` | result value is a *huge* count (≥ 2^20 bytes of `s1` with no match) — exercises the `%zu` conversion of a large `size_t` | prints the full length in decimal, no truncation/overflow | `err_11_huge_length` |
| 12 | `driver` | `s2` = a string that is *longer* than `s1` and shares no byte — O(n·m) worst case, no early exit | prints `strlen(s1)` | `err_12_no_match_long_s2` |

### Boundary conditions required by the task prompt, mapped to this API

| condition | applicability to `driver` | covered by |
|-----------|---------------------------|------------|
| null pointers | both parameters are pointers → rows 1–5 | `error_paths.rs` |
| zero length | `s1 == ""` and/or `s2 == ""` are *valid* zero-length inputs, not errors → rows 2, 4, 10 + `CONFIGS.md` rows 1–4 | both suites |
| oversized length | rows 11–12 (≥ 1 MiB `s1`, long `s2`) | `error_paths.rs` |
| one step past a valid range | there is no numeric range parameter; the analogous case is the byte *value* range — every byte `0x01..=0xFF` is legal in either string and `0x00` terminates. `CONFIGS.md` rows 12–14 sweep the whole byte domain including `0x7F`/`0x80` (the signed-`char` sign-flip boundary, the one place a Rust `i8` vs C `char` comparison could diverge) | `valid_paths.rs` |
| out-of-range enum values across FFI | **not applicable — the API has no enum, integer, flag, or mode parameter.** `driver` takes exactly two `const char *` and returns `void`; `nm -D` confirms `driver` is the only exported symbol. There is no integer input whose value could fall outside a valid variant set. Documented here so the omission is explicit rather than an oversight. The nearest equivalent — arbitrary/never-valid *pointer* values — is covered by rows 1–7. | rows 1–7 |

## Divergences found and fixed (Phase C)

### Divergence 1 — argument access ORDER

The original Rust translation implemented `strcspn` as a nested loop — for each byte
of `s1`, scan `s2` — which means `s2` is **never dereferenced when `s1` is empty**:

```rust
loop { let c = *p; if c == 0 { break; }        // s1 checked first
       let mut q = s2; loop { let d = *q; ... } }   // s2 only reached if s1 non-empty
```

The C library's `strcspn` instead builds a 256-entry reject table from `s2` *before*
looking at `s1`, so `s2` is consumed unconditionally. The difference is directly
observable:

| input | C | Rust (before fix) |
|-------|---|-------------------|
| `driver("", NULL)` | `SIGSEGV` | exits 0, prints `0\n` |
| `driver("", <unterminated s2, 1 byte>)` | `SIGSEGV` | exits 0, prints `0\n` |
| `driver("", <unterminated s2, 4096 bytes>)` | `SIGSEGV` | exits 0, prints `0\n` |
| `driver("", <wild pointer as s2>)` | `SIGSEGV` | exits 0, prints `0\n` |

Probed with `tests/zz_probe.rs` (kept as `tests/access_order.rs`) to establish the C's
real access order instead of assuming it. The probe also pinned down the exact read
*extent*, which the fix had to preserve:

* `s2` is read up to **and including** its NUL, and never past it — a properly
  terminated `s2` sitting flush against an unmapped guard page does **not** fault, at
  reject lengths 0, 1, 2, 3 and 4 (so the fix must not over-read, e.g. must not read
  `s2[1]` when `s2[0]` is already NUL).
* `s1` is read only up to the byte that stops the scan (its NUL or the first rejected
  byte), never past it.

**Fix (first attempt)**: scan `s2` to completion into a 256-entry lookup table first,
then walk `s1`. That reproduced the C's fault behaviour and read extents exactly and
made rows 4 and 7b pass — in a **release** build. It then failed in a debug build, for
the unrelated reason below.

### Divergence 2 — fault SIGNAL differs between build profiles

Running the same suite against `target/debug/libdriver.so` (via `DRIVER_RUST_SO`, see
`check_features.sh`) surfaced a second divergence that the release build hid:

| input | C | Rust debug build |
|-------|---|------------------|
| `driver(NULL, "Z")` | `SIGSEGV` (11) | `SIGABRT` (6) — `thread caused non-unwinding panic` |
| `driver("", NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) |

Since Rust 1.78, `-C debug-assertions` inserts a null/alignment precondition check on
every raw-pointer dereference, so `*p` on a null pointer raises a non-unwinding panic
(which aborts) instead of reaching the hardware and faulting. The C reliably delivers
`SIGSEGV`. No pure-Rust pointer walk can match that in a debug build without dropping
to inline assembly, and `ptr::read_volatile` carries the same precondition check.

**Fix (final)**: `driver` now calls the platform's `strcspn` through
`extern "C"` — the *same* libc function the C code calls — just as the translation
already did for `printf`. Both `strcspn` and `printf` are C standard library
functions rather than part of the translated source, so binding them instead of
reimplementing them is both the faithful reading of the C and the only way to get
identical results, identical read extents, identical access order and identical fault
signals in **every** build profile. As a bonus the Rust `.so`'s dynamic imports now
match the C `.so`'s exactly (`printf@GLIBC_*`, `strcspn@GLIBC_*`).

Both divergences are now covered by permanent regression tests in
`tests/access_order.rs`, which run against the debug **and** release `.so`.
