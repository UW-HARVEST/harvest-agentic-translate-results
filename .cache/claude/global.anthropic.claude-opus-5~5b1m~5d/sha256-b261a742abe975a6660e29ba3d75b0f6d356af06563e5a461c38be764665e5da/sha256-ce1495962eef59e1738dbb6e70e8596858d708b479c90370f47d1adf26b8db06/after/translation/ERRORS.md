# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep results

```
$ grep -n 'return' src/lib.c include/lib.h
src/lib.c:8:        return 1;      # less_than_or_equal -> true  (sort_bits <=)
src/lib.c:10:       return 1;      # less_than_or_equal -> true  (DEAD branch, see #12)
src/lib.c:11:    return 0;         # less_than_or_equal -> false
src/lib.c:34:        return;       # recurse guard: hi - lo <= 1

$ grep -nE 'assert|RETURN_ERROR|errno|NULL|error|ERROR|INT_|MAX|MIN|limits\.h' src/lib.c include/lib.h
(no matches)

$ grep -nE 'if *\(|for|while|switch|case|#if' src/lib.c include/lib.h
src/lib.c:7    if (a->sort_bits <= b->sort_bits)
src/lib.c:9    if (a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id)
src/lib.c:18   for (int k = lo; k < hi; k++)
src/lib.c:19   if (i < split && (j >= hi || less_than_or_equal(a+i, a+j)))
src/lib.c:33   if (hi - lo <= 1)
```

**Key finding:** the library has **NO** error codes, **NO** sentinel returns,
**NO** `assert`, **NO** `NULL` checks and **NO** range checks. `merge_sort`
returns `void`. Its entire rejection surface is therefore:

* one **guard clause** (`hi - lo <= 1` → early `return`), and
* **out-of-domain inputs** that C does not check, where the observable
  "error result" is either *silent no-op*, *silent garbage*, or a *fatal signal*.

The Rust must match all three categories, so every one gets a row. Rows whose
expected C result is a fatal signal are tested by forking a child process and
comparing the **exact** `WTERMSIG` / exit status of C vs Rust — not merely "both
failed somehow".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `merge_sort` | `size == 0`, valid non-null `a`, `b` | `memcpy(b,a,0)` = no-op; `recurse(b,0,0,a)` hits `hi-lo==0 <= 1` → return. **Neither `a` nor `b` is modified at all** (b keeps its pre-fill) | [x] |
| 2 | `merge_sort` | `size == 1`, valid non-null `a`, `b` | `memcpy` copies exactly 16 bytes (incl. padding) `a[0]`→`b[0]`; `recurse` hits `hi-lo==1 <= 1` → return. `a` unchanged; `b[0]==a[0]`; `b[1..]` keeps pre-fill | [x] |
| 3 | `merge_sort` | `size == 0` **and** `a == NULL` **and** `b == NULL` | `memcpy(NULL,NULL,0)`; guard returns. No dereference, no crash, returns normally | [x] |
| 4 | `merge_sort` | `size == -1` / `-2` (small negative), non-null buffers | `sizeof(T)*size` = `(size_t)(int)-1 * 16` → `cltq; shl $4` = `0xFFFFFFFFFFFFFFF0` byte memcpy. **OBSERVED: glibc's `memcpy` returns without copying** for a byte count this large rather than faulting, so control reaches `recurse`, whose `hi-lo < 0` guard returns → silent no-op, `exit 0` | [x] |
| 5 | `merge_sort` | `size == INT_MIN` (`-2147483648`), `INT_MIN+1`, `-1000` | `cltq; shl $4` = `0xFFFFFFF800000000` byte memcpy → **OBSERVED: fatal `SIGSEGV` (signal 11)**, identically in C and Rust. (Which negative values fault vs. silently return is a glibc-internal detail; both sides route to the *same* `memcpy` with the *same* count, so they always agree) | [x] |
| 6 | `merge_sort` | `size < 0` reached *past* the memcpy | `recurse(b,0,size,a)` has `hi-lo = size < 0 <= 1` → guard returns, i.e. negative size is a no-op for the *sort* itself. Directly observable for `size` `-1`/`-2` (row #4), where the buffers come back unchanged | [x] |
| 7 | `merge_sort` | `a == NULL`, `b` non-null, `size > 0` | `memcpy` reads from address 0 → **fatal `SIGSEGV`** | [x] |
| 8 | `merge_sort` | `a` non-null, `b == NULL`, `size > 0` | `memcpy` writes to address 0 → **fatal `SIGSEGV`** | [x] |
| 9 | `merge_sort` | both `a == NULL` and `b == NULL`, `size > 0` | `memcpy` → **fatal `SIGSEGV`** | [x] |
| 10 | `_recurse` | `hi - lo == 1` (leaf of the recursion, every odd sub-range bottom) | early `return`, sub-range left exactly as-is in the *destination* buffer — the recursion relies on the pre-existing `memcpy`'d copy | [x] |
| 11 | `_recurse` | `hi - lo == 0` (only reachable via `size==0` top-level; `split` never produces an empty half for `hi-lo>=2`) | early `return` | [x] |
| 12 | `_less_than_or_equal` | `a->sort_bits > b->sort_bits` — the **only** path that reaches `return 0`. Note line 9's `if` is **DEAD CODE**: it requires `sort_bits ==`, which line 7's `<=` already returned 1 for. So `texture_id` is **never** consulted | returns `0` — and `texture_id` has **zero** influence on the sort order, for *any* input | [x] |
| 13 | `_less_than_or_equal` | `a->sort_bits == b->sort_bits` (tie) | returns `1` via **line 7** (not line 9) → merge takes the left run first → the sort is **stable on `sort_bits`**, and NOT ordered by `texture_id` within a tie | [x] |
| 14 | `_less_than_or_equal` | `sort_bits` at extremes: `INT_MIN` vs `INT_MAX` | plain **signed** `int` comparison (`<=`); `INT_MIN <= INT_MAX` → 1. No unsigned reinterpretation, no overflow (no subtraction is performed) | [x] |
| 15 | `_iteration` | `i >= split` (left run exhausted) | takes `b[k] = a[j]`, `j++` — the `||`-shortcircuit means `less_than_or_equal` is **not** called | [x] |
| 16 | `_iteration` | `i < split && j >= hi` (right run exhausted) | takes `b[k] = a[i]`, `i++`; `less_than_or_equal` is **not** called (short-circuit `||`) | [x] |
| 17 | `_iteration` | `j` reads `a[j]` when `j == hi` is *false* but `hi` exceeds the real array (only if caller lies about `size`) | unchecked out-of-bounds read — not reachable through the public API with a truthful `size`; documented, not tested | n/a |
| 18 | `merge_sort` | `a == b` (same pointer, non-null, `size > 0`) | `memcpy` with `dst == src` (UB in ISO C, no-op in glibc), then the ping-pong merge runs with both buffers aliased → deterministic, *not*-sorted output. Rust must byte-match | [x] |
| 19 | `merge_sort` | `size > INT_MAX/2` so that `lo + hi` overflows in `(lo + hi) / 2` | signed overflow UB; requires ≥ 2^30 elements = 17 GB. Documented, not tested (not allocatable) | n/a |
| 20 | `merge_sort` | struct **padding** bytes (offsets 12..16) non-zero in the input | `memcpy` propagates them verbatim; the `b[k]=a[i]` struct assignment compiles to two 8-byte `mov`s (verified via `objdump`) so padding is **also** propagated by the merge. Rust must propagate identically | [x] |

## Notes on FFI-boundary enum values

`lib.h` declares **no `enum`** and no flag/mode parameter — the only scalar
parameter is `int size`, whose full `int` range is covered by rows 1, 2, 4, 5, 6
and by the Phase B randomized size sweep. There is therefore no
"out-of-range enum variant" input class for this library; the analogous
"value with no valid meaning crossing the FFI boundary" is a negative/oversized
`size`, covered by rows 4–6.

## Test mapping

| `ERRORS.md` row(s) | test in `tests/phase_c_errors.rs` |
|---|---|
| 1 | `err01_size_zero_is_total_noop` |
| 2 | `err02_size_one_copies_exactly_one_element` |
| 3 | `err03_null_pointers_with_size_zero_return_normally` |
| 4, 6 | `err04_06_negative_size_behaves_identically` |
| 5 | `err05_int_min_size_behaves_identically` |
| 7 | `err07_null_source_behaves_identically` |
| 8 | `err08_null_dest_behaves_identically` |
| 9 | `err09_both_null_behaves_identically` |
| 10, 11 | `err10_11_recursion_guard_leaves` |
| 12, 13 | `err12_13_texture_id_never_affects_order` |
| 14 | `err14_signed_key_extremes` |
| 15, 16 | `err15_16_run_exhaustion_paths` |
| 17 | n/a — unreachable through the public API with a truthful `size` |
| 18 | `err18_aliased_buffers` (+ `row27_aliased_buffers`) |
| 19 | n/a — requires ≥ 2^30 elements (17 GB), not allocatable |
| 20 | `err20_padding_propagation` (+ `row23`–`row25`) |
| generic boundary sweep | `boundary_size_domain_sweep_nonfatal` |

## Observed C-vs-Rust termination parity (fatal rows)

Captured from `cargo test --test phase_c_errors -- --nocapture`:

| case | `size` | pointers | C and Rust both |
|---|---|---|---|
| `neg1` | `-1` | valid | `exit 0`, identical buffer digests |
| `neg2` | `-2` | valid | `exit 0`, identical buffer digests |
| `neg1000` | `-1000` | valid | `SIGSEGV` (11) |
| `intmin` | `INT_MIN` | valid | `SIGSEGV` (11) |
| `intmin_plus1` | `INT_MIN+1` | valid | `SIGSEGV` (11) |
| `null_a` | `8` | `a == NULL` | `SIGSEGV` (11) |
| `null_b` | `8` | `b == NULL` | `SIGSEGV` (11) |
| `null_both` | `8` | both `NULL` | `SIGSEGV` (11) |
| `null_both_size1` | `1` | both `NULL` | `SIGSEGV` (11) |

Surviving cases are additionally compared by an FNV-1a digest of BOTH buffers,
so "both survived" can never pass vacuously.
