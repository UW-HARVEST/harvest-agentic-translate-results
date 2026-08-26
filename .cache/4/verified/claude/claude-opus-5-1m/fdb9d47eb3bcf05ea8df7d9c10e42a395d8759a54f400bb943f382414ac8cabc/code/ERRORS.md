# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep census of rejection constructs

Every construct that could reject input was grepped across **all** C sources:

| pattern | hits |
|---|---|
| `return -` | 0 |
| `return NULL` | 0 |
| `RETURN_ERROR` | 0 |
| `assert` / `ASSERT` | 0 |
| `errno` | 0 |
| `ERROR` | 0 |
| `enum` | 0 |
| `_MAX` / `_MIN` | 0 |
| `if ( !` | 0 |
| `== NULL` / `!= NULL` | 0 |
| `exit(` / `abort(` | 0 |
| `perror` / `fprintf` | 0 |

Complete list of `return` statements in the library (`grep -rn return`):

```
c_src/src/lib.c:8:        return 1;     <- comparator "true"
c_src/src/lib.c:10:       return 1;     <- comparator "true" (dead branch)
c_src/src/lib.c:11:    return 0;        <- comparator "false"
c_src/src/lib.c:34:        return;      <- recursion base case, void
```

Complete list of `if` statements (`grep -rn 'if ('`):

```
c_src/src/lib.c:7:   if (a->sort_bits <= b->sort_bits)
c_src/src/lib.c:9:   if (a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id)
c_src/src/lib.c:19:  if (i < split && (j >= hi || ..._less_than_or_equal(a + i, a + j)))
c_src/src/lib.c:33:  if (hi - lo <= 1)
```

## Conclusion of the census

**This library has no explicit error surface.** `merge_sort` returns `void`,
performs no validation of `a`, `b` or `size`, has no error codes, no sentinel
returns, no asserts, no enums, and no documented valid range. The only `if`s are
algorithmic (comparator result, run-exhaustion, recursion base case) — none of
them reject input.

Consequently the error-path table below enumerates the **implicit** rejection /
degenerate / boundary behaviours: the conditions under which the C silently
does nothing, and the boundary and out-of-contract values that a caller can
actually pass across the FFI boundary. Each row is a differential test that
asserts C and Rust behave *identically*, including identical crash signals for
the out-of-contract rows.

## Error / boundary surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `merge_sort` | `size == 0`, valid non-null `a`, `b` | `memcpy(b,a,0)` is a no-op; `recurse` hits `hi-lo=0 <= 1` and returns. **Neither buffer is written.** No crash. | `err_e1_size_zero_no_writes` | [x] |
| E2 | `merge_sort` | `size == 0`, `a == NULL`, `b == NULL` | `memcpy(NULL,NULL,0)` — glibc no-ops for `n==0`; `recurse` returns before any deref. **No crash, no writes.** Rust must also not deref (it skips the copy when `bytes == 0`). | `err_e2_size_zero_null_pointers` | [x] |
| E3 | `merge_sort` | `size == 1` (recursion base case, no merge ever runs) | `memcpy` copies 16 bytes `a`→`b`; `recurse` hits `hi-lo == 1 <= 1` and returns. `a` unchanged, `b == a` byte-for-byte incl. padding. | `err_e3_size_one_base_case` | [x] |
| E4 | `merge_sort` | `size == 1`, `b == NULL` (undersized/NULL scratch) | `memcpy(NULL, a, 16)` writes to address 0 → **SIGSEGV**. Out of contract; both impls must fault identically. | `err_e4_null_scratch_faults_identically` | [x] |
| E5 | `merge_sort` | `size == 1`, `a == NULL`, valid `b` | `memcpy(b, NULL, 16)` reads address 0 → **SIGSEGV**. Both impls must fault identically. | `err_e5_null_source_faults_identically` | [x] |
| E6 | `merge_sort` | `size < 0` — the `int`→`size_t` sign-extension trap. Sub-cases: **small** negatives (`-1`, `-2`, `-3`) and **large** negatives (`-1000`, `-65536`, `-2^20`) | `sizeof(spritebatch_sprite_t) * size` converts `int`→`size_t`: gcc emits `cltq; shl $0x4` (sign-extend, then ×16), so `-1` becomes `0xFFFFFFFFFFFFFFF0` bytes. Rust computes `(size as usize).wrapping_mul(16)` = the *identical* byte count. **Empirically measured** (12 runs each, statuses perfectly stable): small negatives **do NOT fault** — glibc's `memmove` sees the wrapped length as an overlap, copies backward from a wrapped address and the process **survives with exit code 0**; large negatives **do** fault with **SIGSEGV**. Both implementations must reproduce whichever outcome the C produces, per sub-case. | `err_e6_negative_size_faults_identically` | [x] |
| E7 | `merge_sort` | `size == INT_MIN` (most extreme negative; `-size` is not representable) | Same sign-extension path: `0x8000000000000000` bytes → **SIGSEGV** (measured, stable). | `err_e7_int_min_size_faults_identically` | [x] |
| E8 | `merge_sort` | `size` one step past the real buffer length (buffer of `n`, `size = n+1`) | No bounds check exists → reads/writes one element out of bounds. Both impls must perform the *same* out-of-bounds access pattern and produce the same result. Tested safely with generous slack allocation (guard region) so the OOB element is addressable. | `err_e8_size_one_past_buffer` | [x] |
| E9 | `merge_sort` | `a` and `b` are the **same** pointer (overlap; `memcpy` contract violation) | `memcpy(b,a,n)` with `b == a` is UB but glibc no-ops for exact overlap; the sort then reads and writes one buffer. Whatever the C produces, Rust must produce bit-identically. | `err_e9_aliased_buffers` | [x] |
| E10 | `merge_sort` | `a` and `b` partially overlapping (`b == a + 1`) | Overlapping `memcpy` — UB; direction-dependent. Both impls must produce the same bytes. | `err_e10_partially_overlapping_buffers` | [x] |
| E11 | comparator (via `merge_sort`) | `sort_bits == INT_MIN` / `INT_MAX` mixed — signed comparison at the extremes | `a->sort_bits <= b->sort_bits` is a **signed** compare; `INT_MIN <= anything` is true. No overflow occurs (no subtraction is used). Must match. | `err_e11_sort_bits_signed_extremes` | [x] |
| E12 | comparator (via `merge_sort`) | `texture_id == 0` / `u64::MAX` — unsigned compare in the **dead** second branch | Line 9 is unreachable (line 7 already returns 1 whenever `sort_bits` are equal), so `texture_id` **never** influences ordering. Rust must reproduce this quirk, i.e. `texture_id` must be ignored as a sort key. | `err_e12_texture_id_never_affects_order` | [x] |
| E13 | `merge_sort` | out-of-range **enum** value across the FFI boundary | **N/A — the API contains no enum type.** `grep -c enum` = 0 over all C sources; the only scalar parameter is `int size`, and every one of the 2^32 `int` values is accepted without validation (rows E1/E3/E6/E7 cover its boundaries: `0`, `1`, `-1`, `INT_MIN`, and E8 covers `n+1`). Recorded so the row is explicitly discharged rather than silently skipped. | `err_e13_int_size_domain_sweep` | [x] |
| E14 | `merge_sort` | `size == 2` with `b` valid but only 1 element allocated → the *scratch* buffer is the one overrun | No check; writes `b[1]` out of bounds. Both must behave identically (tested with guard slack). | `err_e8_size_one_past_buffer` (shared) | [x] |
| E15 | internal `recurse` | `(lo + hi)` signed-overflow at `size` near `INT_MAX` | `int split = (lo + hi) / 2` overflows for `hi` near `INT_MAX` → UB in C. gcc -O0 emits plain two's-complement `add %edx,%eax` followed by round-toward-zero division (`shr $31; add; sar $1`); Rust uses `wrapping_add(hi) / 2`, which is bit-identical. **Not runtime-testable** — it needs `INT_MAX * 16 B ≈ 34 GB` of allocation. Discharged by codegen inspection, documented here. | n/a (documented, allocation-infeasible) | [x] |

## Measured outcomes (all statuses identical C vs Rust)

Recorded from `cargo test --test phase_c_errors -- --nocapture`:

| trigger | signal | exit code | memory compared? |
|---|---|---|---|
| `size` 0..=16 (in allocation) | – | 0 | **yes**, byte-for-byte |
| `size = 0`, both pointers NULL | – | 0 | yes |
| `size = 17, 100, 4096` (past the buffer) | **6** (`SIGABRT`, glibc heap-corruption detector fires in `free()`) | – | status only |
| `size = 2^20, 2^28, INT_MAX-1, INT_MAX` | **11** (`SIGSEGV`) | – | status only |
| `size = -1, -2, -3` | – | **0** (survives!) | status only (post-state is ASLR-dependent) |
| `size = -1000, -65536, -2^20, INT_MIN+1, INT_MIN` | **11** (`SIGSEGV`) | – | status only |
| `b == NULL, size = 1` | **11** | – | status only |
| `a == NULL, size = 1` | **11** | – | status only |

For `size = -1` the *smeared* memory is not a well-defined observable (it
depends on ASLR; two runs of the **same** C library disagree). Under
`setarch -R` (ASLR disabled) the C and Rust post-states were verified
byte-identical, which confirms both hand `memcpy` the same length; the suite
asserts only the stable part, the termination status.

## Notes on how the crash rows are tested

Rows E4–E7 are genuine faults on **both** sides. The test harness re-executes
the test binary as a child process (`std::process::Command` on
`current_exe()`, selected by an env var), loads one implementation, performs
the offending call, and reports the child's termination signal. The row passes
only when the C child and the Rust child terminate with the **same** signal
(e.g. both `SIGSEGV`/11) — not merely "both failed somehow".
