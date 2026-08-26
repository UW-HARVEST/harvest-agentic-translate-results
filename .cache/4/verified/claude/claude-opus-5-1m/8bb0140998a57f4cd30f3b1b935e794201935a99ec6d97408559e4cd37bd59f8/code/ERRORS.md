# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Grep audit of every rejection
construct the C source contains:

```
grep -c 'RETURN_ERROR\|return -1\|return NULL\|assert\|errno\|== NULL\|!= NULL' c_src/src/lib.c   -> 0
grep -c '#if\|#ifdef'                                                            c_src/src/lib.c   -> 0
grep -n 'if ('                                                                   c_src/src/lib.c   -> 2 sites (L67, L111)
grep -n 'for ('                                                                  c_src/src/lib.c   -> 5 sites (L81, L86, L116, L146, L161)
grep -n 'malloc'                                                                 c_src/src/lib.c   -> 3 sites (L79, L145, L159) all UNCHECKED
grep -n 'op(a, b, c)'                                                            c_src/src/lib.c   -> 1 indirect call (L44) UNCHECKED
```

**This library has no error codes, no error enums, no `assert`, no `errno`, and
no NULL checks whatsoever.** Consequently every "rejection" is one of:

* **`no-op`** — a guard conjunct is false, so the mutation is silently skipped
  and the buffer is left byte-identical (`void` return).
* **`returns 0`** — a loop guard is immediately false, so the accumulator is
  never touched and the function returns `0`.
* **`wraps`** — signed-integer overflow. UB in ISO C, but both `gcc -O` on
  x86-64 and the Rust `wrapping_*` translation produce two's-complement wrap;
  the required behaviour is *identical wrap*, verified differentially.
* **`SIGSEGV`** — an unchecked dereference / unchecked indirect call. Tested by
  running the call in a forked child (re-exec of the test binary) and asserting
  **both** libraries die from the **same signal**.

Every row below has a differential test in `tests/error_paths.rs`
(`SIGSEGV` rows in `tests/crash_parity.rs`). `[x]` = test written **and** passing
against both `.so`s.

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `shift_array_data` | `shift_by == 0` — L67 conjunct `shift_by > 0` false | no-op; `arr` byte-identical | [x] |
| E2 | `shift_array_data` | `shift_by < 0` (e.g. `-1`) — L67 conjunct `shift_by > 0` false | no-op; `arr` byte-identical | [x] |
| E3 | `shift_array_data` | `shift_by == INT_MIN` — same conjunct false | no-op; `arr` byte-identical | [x] |
| E4 | `shift_array_data` | `shift_by == size` — L67 conjunct `shift_by < size` false | no-op; `arr` byte-identical | [x] |
| E5 | `shift_array_data` | `shift_by > size` (e.g. `size+1`) — same conjunct false | no-op; `arr` byte-identical | [x] |
| E6 | `shift_array_data` | `shift_by == INT_MAX`, `size < INT_MAX` — same conjunct false | no-op; `arr` byte-identical | [x] |
| E7 | `shift_array_data` | `size == 0` (zero length) with any `shift_by` | no-op; nothing written | [x] |
| E8 | `shift_array_data` | `size < 0` (negative length, e.g. `-4`) | no-op; nothing written | [x] |
| E9 | `shift_array_data` | `size == 1` — no `shift_by` can satisfy `0 < shift_by < 1` | no-op for every `shift_by` | [x] |
| E10 | `shift_array_data` | `arr == NULL` **and** guard false (`shift_by <= 0` or `>= size`) | returns normally, NULL never dereferenced | [x] |
| E11 | `shift_array_data` | `arr == NULL` **and** `0 < shift_by < size` → `memmove(NULL,...)` | fatal `SIGSEGV` | [x] |
| E12 | `process_pointer_data` | `ptr == NULL` → L74 `*ptr` | fatal `SIGSEGV` | [x] |
| E13 | `process_pointer_data` | `value * multiplier` overflows `int` (e.g. `INT_MAX * 2`) | wraps (two's complement), then `+ global_accumulator` also wraps | [x] |
| E14 | `compute_with_dynamic_memory` | `count == 0` (zero length) → both loops' guard `i < 0` false | returns `0`; `malloc(0)` result unused | [x] |
| E15 | `compute_with_dynamic_memory` | `count < 0` → `count*sizeof(int)` converts to a huge `size_t`, `malloc` returns NULL, but both loop guards are false so NULL is **never** dereferenced | returns `0` (no crash) | [x] |
| E16 | `compute_with_dynamic_memory` | `count == INT_MIN` (extreme negative) | returns `0` (no crash) | [x] |
| E17 | `compute_with_dynamic_memory` | `base + i*3` / `sum +=` overflow `int` (e.g. `base = INT_MAX`, `count = 1000`) | wraps | [x] |
| E18 | `get_time_based_value` | `seed * 3600` overflows `int` (e.g. `seed = 1_000_000`) | wraps to `int`, sign-extended to `time_t`; result `wrap(seed*3600)/100 + seed` | [x] |
| E19 | `get_time_based_value` | `seed == INT_MAX` | wraps; `+ seed` also wraps | [x] |
| E20 | `get_time_based_value` | `seed == INT_MIN` (`seed*3600` wraps to `0`) | `(int)(0.0/100) + INT_MIN == INT_MIN` | [x] |
| E21 | `get_time_based_value` | negative `diff/100` — truncation must be **toward zero**, not floor (e.g. `seed = 1_000_000` → `-6949672.96`) | truncates toward zero (`-6949672`) | [x] |
| E22 | `manipulate_records` | `shift == 0` — L111 conjunct `shift > 0` false | no `memmove`; loop runs `num_records` times; returns sum of all `.value` | [x] |
| E23 | `manipulate_records` | `shift < 0` — L111 conjunct false **and** L116 bound `num_records - shift > num_records` → reads **past the end** of the caller array | no `memmove`; sums `num_records - shift` elements incl. out-of-bounds ones (reproduced verbatim) | [x] |
| E24 | `manipulate_records` | `shift == num_records` — L111 conjunct `shift < num_records` false, L116 bound `0` | no `memmove`; returns `0` | [x] |
| E25 | `manipulate_records` | `shift > num_records` — L116 bound negative | no `memmove`; returns `0` | [x] |
| E26 | `manipulate_records` | `num_records == 0`, `shift == 0` (zero length) | returns `0`; array untouched | [x] |
| E27 | `manipulate_records` | `num_records < 0` (negative length) with `shift == 0` → bound negative | returns `0`; array untouched | [x] |
| E28 | `manipulate_records` | `num_records - shift` overflows `int`: `num_records == INT_MIN`, `shift == INT_MIN` → guard `shift > 0` false, bound `0` | returns `0` | [x] |
| E29 | `manipulate_records` | `total +=` overflows `int` (values near `INT_MAX`) | wraps | [x] |
| E30 | `manipulate_records` | `records == NULL` **and** loop bound `<= 0` (`shift >= num_records`) | returns `0`, NULL never dereferenced | [x] |
| E31 | `manipulate_records` | `records == NULL` **and** loop bound `> 0` | fatal `SIGSEGV` | [x] |
| E32 | `apply_operation` | `op == NULL` → L44 indirect call through a null function pointer. (This is the C-enum analogue: the parameter is an *address*, so any bit pattern is accepted at the ABI level with no validation.) | fatal `SIGSEGV` | [x] |
| E33 | `apply_operation` | `op` = a **non-function, non-executable data address** (bit pattern with no valid target) | fatal `SIGSEGV` | [x] |
| E34 | `apply_operation` | `op` = valid callback, but `op` itself overflows (`add_three(INT_MAX,1,1)` via `apply_operation`) | wraps; `apply_operation` adds no checks | [x] |
| E35 | `add_three` | `a + b + c` overflows `int` (e.g. `INT_MAX, 1, 0`) | wraps | [x] |
| E36 | `multiply_add` | `a * b` overflows `int` (e.g. `INT_MIN, -1, 0`) | wraps | [x] |
| E37 | `complex_calc` | `a - b` overflows `int` (e.g. `INT_MIN, 1, 1`) | wraps | [x] |
| E38 | `complex_calc` | `(a-b) * c + global_counter` overflows `int` | wraps | [x] |
| E39 | `increment_counter` | `global_counter += value` overflows `int` (repeat `INT_MAX`) | wraps; persists in the `.so`'s `static` | [x] |
| E40 | `update_accumulator` | `global_accumulator * 2 + value` overflows `int` | wraps; persists | [x] |
| E41 | `hatch` | every accumulation in `hatch` overflows (`INT_MAX` / `INT_MIN` params) | wraps; returns a defined wrapped `int`, no crash | [x] |
| E42 | `hatch` | `param3 == INT_MIN` → internal `get_time_based_value(INT_MIN)` extreme | wraps; no crash | [x] |

## Generic FFI boundary conditions (required even though not in the C's own checks)

| # | condition | covered by |
|---|-----------|-----------|
| G1 | NULL pointer for every pointer parameter | E10/E11 (`arr`), E12 (`ptr`), E30/E31 (`records`), E32 (`op`) |
| G2 | zero length | E7 (`size==0`), E14 (`count==0`), E26 (`num_records==0`) |
| G3 | negative / "oversized" length | E8, E15, E16, E27 |
| G4 | one step past a valid range | E4 (`shift_by==size`), E5 (`size+1`), E24 (`shift==num_records`), E25 (`num_records+1`), E23 (`shift==-1`) |
| G5 | out-of-range "enum"-like value across FFI | the API declares **no `enum`** (`grep -c enum c_src/src/lib.c` → 0). The only non-scalar-domain parameter is the `operation_func` function pointer, whose invalid bit patterns are E32 (null) and E33 (non-executable address). |
| G6 | extreme scalars `INT_MIN` / `INT_MAX` for every `int` parameter | E3, E6, E16, E19, E20, E28, E35–E38, E41, E42 |

## Documented but deliberately NOT differentially tested

| condition | why |
|---|---|
| `compute_with_dynamic_memory` with `count` so large that `malloc` genuinely fails while overcommit lets it succeed (e.g. `count = INT_MAX` → 8 GiB) | outcome depends on host RAM/overcommit and can trigger the OOM killer; not reproducible, and C would deref NULL. Values up to `count = 1<<22` (16 MiB) *are* tested in `CONFIGS.md` row C24. |
| `shift_array_data` with `size = INT_MAX` on a small buffer | `memmove` of ~8 GiB out of a small buffer; unbounded wild write, host-dependent. |
| `manipulate_records` with `shift < 0` and `|shift|` huge (e.g. `shift = INT_MIN`, `num_records = -1` → bound `INT_MAX`) | a 2^31-iteration wild read; host-dependent. The *bounded* out-of-bounds-read form is tested in E23 with a padded buffer so the read is deterministic for both libraries. |

## Results

```
$ cargo test --test error_paths -- --nocapture
ERRORS.md (non-fatal): all 37 rows exercised
test phase_c_nonfatal_error_rows ... ok

$ cargo test --release --test crash_parity -- --nocapture --test-threads=1
strict signal parity: true
E11 [shift_array_null]      C=Signal(11)  Rust=Signal(11)  (strict=true)
E12 [ppd_null]              C=Signal(11)  Rust=Signal(11)  (strict=true)
E31 [records_null_shift0]   C=Signal(11)  Rust=Signal(11)  (strict=true)
E31 [records_null_shift2]   C=Signal(11)  Rust=Signal(11)  (strict=true)
E32 [apply_null]            C=Signal(11)  Rust=Signal(11)  (strict=true)
E33 [apply_data_addr]       C=Signal(11)  Rust=Signal(11)  (strict=true)
ERRORS.md (fatal): all 5 rows exercised
test phase_c_fatal_error_rows ... ok
```

**42/42 rows pass** (37 non-fatal + 5 fatal), in the debug and release profiles,
under `--no-default-features` and the default feature set.

`tests/doc_coverage.rs::errors_md_rows_match_the_tests` asserts the row ids here
are exactly `ERROR_ROWS_NONFATAL ∪ ERROR_ROWS_FATAL`, that the two sets are
disjoint, and that the ids run `E1..=E42` with no gaps.

### Profile caveat for the five fatal rows

`std::ptr::read` / `ptr::copy` / `ptr::write_bytes` carry
`assert_unsafe_precondition!` null/alignment checks that are compiled in **only**
when the crate is built with `debug-assertions = on`. Measured signals:

| row | C `.so` | Rust `.so` (release, = shipped artifact) | Rust `.so` (debug) |
|---|---|---|---|
| E11 `shift_array_data(NULL,10,3)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGABRT (6) — `ptr::copy` UB-check |
| E12 `process_pointer_data(NULL,3)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGSEGV (11) |
| E31 `manipulate_records(NULL,5,0)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGSEGV (11) |
| E31 `manipulate_records(NULL,5,2)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGABRT (6) — `ptr::copy` UB-check |
| E32 `apply_operation(NULL,…)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGSEGV (11) |
| E33 `apply_operation(<data addr>,…)` | SIGSEGV (11) | SIGSEGV (11) ✔ | SIGSEGV (11) |

Exact-signal parity therefore holds for the artifact that actually ships. When
the `.so` under test still has UB-checks enabled, `crash_parity.rs` detects that
(by probing) and asserts the weaker guarantee "both die from a fatal signal and
neither completes the call", printing both signals. This is a build-profile
artifact of the Rust standard library's debug checks, **not** a translation
difference: the injected-bug check below confirms no real divergence exists.

## Mutation adequacy

`ERRORS.md` + `CONFIGS.md` rows passing is only meaningful if the tests can
actually see a divergence. `mutation_check.sh` injects 22 deliberate bugs into
`src/lib.rs`, rebuilds the `cdylib`, and requires the suite to fail each time:

```
=== 22/22 mutants killed ===
MUTATION CHECK PASSED
```

Mutants cover: `complex_calc` state sign, `update_accumulator` factor,
`get_time_based_value` truncation direction and the `3600` constant,
`compute_with_dynamic_memory` stride, both `shift_array_data` guard bounds and
its `memset`, `manipulate_records` loop bound / guard / `memmove` source offset /
`memmove` count, the `DataRecord` `time_t` width (48→40-byte stride),
`increment_counter` / `process_pointer_data` state signs, `add_three`,
`multiply_add`, and five separate constants inside `hatch`.

One further mutant, `manipulate_records`' guard `shift > 0` → `shift >= 0`, was
tried and **survived**. It was analysed and is an *equivalent mutant*, not a
blind spot: with `shift == 0` the extra branch executes
`memmove(records, records + 0, n)` — a copy of the buffer onto itself, which is
unobservable; for `shift < 0` the `shift >= 0` conjunct still gates it out. It
was therefore replaced by three observable mutations of the same branch
(`shift > 1`, `memmove` source `shift+1`, `memmove` count `n-shift-1`), all of
which are killed.
