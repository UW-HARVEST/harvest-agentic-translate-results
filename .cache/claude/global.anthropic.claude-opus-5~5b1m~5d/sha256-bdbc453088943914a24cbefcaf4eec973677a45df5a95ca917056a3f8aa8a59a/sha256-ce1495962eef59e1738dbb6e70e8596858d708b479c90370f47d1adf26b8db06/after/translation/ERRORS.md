# ERRORS.md — Phase A error-surface table

## Mechanical derivation

Grepped `c_src/src/driver.c` and `c_src/include/driver.h` for every rejection
construct:

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|if *\(|switch|#if|MAX|MIN|<=|>=' \
  c_src/src/driver.c c_src/include/driver.h
```

Result: the only hit is the `#ifndef DRIVER_H_` include guard in the header.

Therefore the C library has, **mechanically**:

- 0 `RETURN_ERROR`-style macros
- 0 `return -1` / `return NULL` / error-enum returns (both public functions are
  `void` and contain no `return` statement at all)
- 0 `assert()` calls
- 0 explicit range checks, null checks, or min/max constants
- 0 enum parameters (so there is no out-of-range-enum class of input)

The entire error surface is therefore **implicit**: it consists of the boundary
and out-of-contract conditions the two `void` functions reach by falling
through their loop guards, plus the undefined behaviour reached for
out-of-contract arguments. Every row below is derived from a specific construct
in the C source, cited by line.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `fma_array` | `len == 0`; loop guard `i < len` at `driver.c:30` is false on entry | returns normally, writes nothing to `out`, `out` unmodified | `err_e1_fma_len_zero` |
| E2 | `fma_array` | `len < 0` (e.g. `-1`, `-5`, `INT_MIN`); guard `0 < len` at `driver.c:30` false | returns normally, no writes, no crash — silent no-op (NOT an error return) | `err_e2_fma_len_negative` |
| E3 | `fma_array` | `len == 0` **and** every pointer `NULL` | returns normally, pointers never dereferenced, no crash | `err_e3_fma_len_zero_all_null` |
| E4 | `fma_array` | `len < 0` **and** every pointer `NULL` | returns normally, pointers never dereferenced, no crash | `err_e4_fma_len_negative_all_null` |
| E5 | `fma_array` | `out == NULL`, `len > 0` — unguarded store `out[i] = …` at `driver.c:31` | UB: SIGSEGV. Out of contract; no error code exists to compare. Documented, not asserted. | documented (see below) |
| E6 | `fma_array` | any of `mul1`/`mul2`/`add` `NULL`, `len > 0` — unguarded loads at `driver.c:31` | UB: SIGSEGV. Out of contract. | documented (see below) |
| E7 | `fma_array` | signed-overflow of `mul1[i] * mul2[i]` at `driver.c:31` (e.g. `INT_MAX * INT_MAX`) | C signed overflow is UB, but the shipped `.so` (no `-O`, no `-ftrapv`) emits `imul`: two's-complement wrap. Verified: `INT_MAX*2+1 == -1`. Must be reproduced, not "fixed". | `err_e7_fma_mul_overflow` |
| E8 | `fma_array` | signed-overflow of `… + add[i]` at `driver.c:31` (e.g. `1 + INT_MAX`) | same: two's-complement wrap via `add`. Verified: `INT_MAX*INT_MAX+INT_MAX == INT_MIN`. | `err_e8_fma_add_overflow` |
| E9 | `fma_array` | `len` larger than the caller's real buffers | UB: out-of-bounds read/write. The C performs **no** length validation — there is no length to validate against. Out of contract. | documented (see below) |
| E10 | `driver` | `len == 0`; VLA `int out[0]` at `driver.c:43`, `memcpy(…, 0)` at `:44`, then both loops in `inner` fall through | returns normally, prints **nothing** (empty stdout), no crash | `err_e10_driver_len_zero` |
| E11 | `driver` | `len == 0` **and** `data == NULL`; `memcpy(out, NULL, 0)` at `driver.c:44` | returns normally, prints nothing, no crash | `err_e11_driver_len_zero_null_data` |
| E12 | `driver` | `len < 0`; `int out[len]` (negative VLA size) at `driver.c:43` **and** `len * sizeof(int)` at `:44` where `int len` converts to `size_t`, so `-1` becomes `18446744073709551612` → `memcpy` of ~16 EiB | UB with **no reproducible result**: measured across lengths the C variously SIGSEGVs, takes SIGBUS, runs forever, or returns cleanly (table below). No error code exists. Rust returns a benign no-op. | `err_e12_driver_len_negative` (subprocess) |
| E13 | `driver` | `len > 0` but `data == NULL`; `memcpy(out, NULL, len*4)` at `driver.c:44` | UB: SIGSEGV. The C performs **no** null check. Out of contract. | documented (see below) |
| E14 | `driver` | `len` so large the VLA exceeds the stack (e.g. `1000000` → 4 MB, or `1<<24`) at `driver.c:43` | UB: stack overflow → SIGSEGV (verified: `driver(d, 1000000)` dumps core). No `alloca` failure check exists. Rust heap-allocates instead. Documented below. | `err_e14_driver_len_stack_overflow` (subprocess) |
| E15 | `driver` | `len` shorter than the real `data` buffer (valid) vs longer (invalid) — `memcpy` length comes solely from the argument at `driver.c:44` | UB on over-long `len`: out-of-bounds read. No validation. Out of contract. | documented (see below) |
| E16 | `inner` (reached only via `driver`) | full self-aliasing `fma_array(out, out, out, out, len)` at `driver.c:36` — `out` is simultaneously the output and all three inputs, which violates the `const`-qualified parameters' implied non-aliasing | NOT an error in practice: the loop is a forward, element-local read-modify-write, so each element becomes `x*x + x` deterministically. Must be reproduced exactly. | `err_e16_inner_self_aliasing` |

## Rows E5, E6, E9, E13, E15 — why they are documented rather than asserted

These rows are all the same class: the C dereferences a caller-supplied pointer
with **no** null or length check, so the "expected C result" is a SIGSEGV, not a
comparable error code or sentinel. The instruction for Phase C is to assert both
sides return *the same error code or sentinel*; here no such value exists on
either side, and deliberately faulting the test process on both sides would only
assert "both crashed somehow", which is explicitly not sufficient. They are
recorded here for completeness of the error surface and are covered indirectly:
E5/E6 are exercised in their *defined* form by E3/E4 (null pointers that the
loop guard makes unreachable), and E9/E15 are exercised in their defined form by
the exact-length rows in `CONFIGS.md`.

## Rows E12 and E14 — documented UB divergence (measured, not assumed)

### E12 — `driver(data, len)` with `len < 0`

`len * sizeof(int)` at `driver.c:44` promotes the negative `int` to `size_t`, so
`memcpy` is handed a ~1.8e19-byte length. Measured with a native C probe
(`gcc -O0`, linked against the built `libdriver.so`):

| `len` | computed `memcpy` size | C disposition |
|-------|------------------------|---------------|
| `-1` | `18446744073709551612` | SIGSEGV (139) |
| `-2` | `18446744073709551608` | SIGSEGV |
| `-3`, `-4`, `-7`, `-8` | ~1.8e19 | SIGSEGV |
| `-16` | `18446744073709551552` | **hangs** (killed by watchdog) |
| `-100` | ~1.8e19 | SIGSEGV |
| `-1000` | `18446744073709547616` | **exits 0, prints nothing** |
| `-10000`, `-100000`, `-1000000` | ~1.8e19 | SIGSEGV |
| `INT_MIN` | ~1.8e19 | **SIGBUS** (135) |
| `INT_MIN + 1` | ~1.8e19 | SIGSEGV |

The disposition is stable for a given caller/stack layout but is otherwise
arbitrary — crash, hang, and clean return all occur. **The C therefore has no
result for a negative `len` that anything could be compared against.**

`err_e12_driver_len_negative` accordingly asserts what is actually verifiable:

1. the Rust side is deterministic, benign and silent for every negative `len`
   (never a crash, never a hang);
2. for every `len` where the C *does* return normally (e.g. `-1000`), the Rust
   also returns normally and, like the C, prints nothing — the only case where a
   differential comparison is meaningful;
3. the C still faults for at least one `len`, so that this table is guarded
   against silently going stale.

### E14 — `driver(data, len)` with a stack-overflowing `len`

`int out[len]` at `driver.c:43` is a VLA with no stack-clash probing, so a large
`len` moves the stack pointer past the guard gap. Measured:

| `len` | VLA size | C disposition |
|-------|----------|---------------|
| `1024` | 4 KiB | exits 0 |
| `4096` | 16 KiB | exits 0 (this is the largest size Phase B exercises) |
| `16384` | 64 KiB | SIGSEGV |
| `65536` … `16777216` | 256 KiB … 64 MiB | SIGSEGV |
| `1000000` | 4 MiB | SIGSEGV (used by the test) |

Unlike E12 this is stable, so `err_e14_driver_len_stack_overflow` asserts the C
is killed by a signal while the Rust — which heap-allocates instead of using a
VLA — returns cleanly.

### Summary of the divergence

| input | C `.so` | Rust `.so` |
|-------|---------|-----------|
| `driver(data, len)`, `len < 0` | SIGSEGV / SIGBUS / hang / clean return, depending on `len` | returns, prints nothing |
| `driver(data, 1000000)` | SIGSEGV (4 MiB VLA overflows the stack) | returns, prints 1000000 lines |

These two rows are the **only** behavioural divergences in the library, and both
lie strictly inside undefined behaviour where the C `.so` produces no result to
match. For every input the C defines, the two `.so`s agree byte-for-byte
(Phases B and C).

Note: the C stack overflow in E14 is triggered inside a forked child, and the
Rust runtime's segfault handler in that child prints
`thread ... has overflowed its stack / fatal runtime error: stack overflow` to
stderr before dying. That message is expected test output, not a failure.

## Completion checklist (Phase C)

- [x] E1 — `fma_array`, `len == 0`
- [x] E2 — `fma_array`, `len < 0`
- [x] E3 — `fma_array`, `len == 0` + all-`NULL`
- [x] E4 — `fma_array`, `len < 0` + all-`NULL`
- [x] E5 — documented (unguarded store, `out == NULL`, UB)
- [x] E6 — documented (unguarded loads, input `NULL`, UB)
- [x] E7 — `fma_array`, multiply overflow wrap
- [x] E8 — `fma_array`, add overflow wrap
- [x] E9 — documented (no length validation exists, UB)
- [x] E10 — `driver`, `len == 0`
- [x] E11 — `driver`, `len == 0` + `data == NULL`
- [x] E12 — `driver`, `len < 0` (UB divergence pinned via subprocess)
- [x] E13 — documented (`data == NULL`, `len > 0`, UB)
- [x] E14 — `driver`, stack-overflowing `len` (UB divergence pinned via subprocess)
- [x] E15 — documented (no length validation exists, UB)
- [x] E16 — `inner` full self-aliasing via `driver`

## Generic boundaries required by Phase C, and where they are covered

| boundary | covered by |
|----------|-----------|
| null pointers | E3, E4 (defined form); E5, E6, E13 (documented UB form) |
| zero length | E1, E3, E10, E11 |
| oversized length | E12, E14 (pinned); E9, E15 (documented) |
| one step past a valid range | E2 (`len = -1`, one below the valid `0`); `cfg` rows `len = 1` / `INT_MIN` / `INT_MAX` element values |
| out-of-range enum across FFI | **N/A — the C API declares no enum parameter.** `driver.h` exposes only `const int *` and `int`. Every `int` bit pattern is a representable input and is covered by the randomized `i32` rows in `CONFIGS.md` plus E2/E7/E8. |
