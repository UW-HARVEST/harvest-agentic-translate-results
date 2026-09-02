# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c`. Exhaustive grep for every
rejection construct:

```sh
grep -n 'abort\|return\|assert\|NULL\|-1\|if \|else \|switch\|#if' c_src/src/lib.c c_src/include/lib.h
```

Result — the C source contains exactly:

* `c_src/src/lib.c:12` — one `if` with **two** `||`-joined reject conditions
* `c_src/src/lib.c:13` — `abort();` (the one and only failure action)
* `c_src/src/lib.c:26` — `return hex;` (the one and only success return)

There are **no** `assert`s, **no** error enums, **no** `return NULL`, **no**
`return -1`, **no** null-pointer checks, and **no** other range checks in the
library. The entire error surface is the single `abort()` reachable through two
distinct triggers. `abort()` raises `SIGABRT` (signal 6) and does not return, so
the "expected C result" for every row below is process termination by `SIGABRT`.

The two magic constants in the guard are the only numeric limits in the source:

| constant | value | meaning |
|----------|-------|---------|
| `(18446744073709551615UL) / 2` | `9223372036854775807` = `0x7FFF_FFFF_FFFF_FFFF` | inclusive upper bound rejected for `bin_len` |
| `bin_len * 2U` | wrapping `size_t` product | inclusive lower bound rejected for `hex_maxlen` |

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `bin2hex` | `bin_len >= 9223372036854775807` — first `\|\|` operand true. Tested at the exact boundary `bin_len == 0x7FFF_FFFF_FFFF_FFFF` with `hex_maxlen == usize::MAX` (so the *second* operand is false: `usize::MAX > bin_len*2`), proving the first check alone fires. | `abort()` → `SIGABRT` | [x] |
| E2 | `bin2hex` | `bin_len > 9223372036854775807`, e.g. `bin_len == 0x8000_0000_0000_0000` and `bin_len == usize::MAX`, with `hex_maxlen == usize::MAX`. Note `usize::MAX * 2` wraps to `0xFFFF…FE`, so for `bin_len == usize::MAX` the second operand is *also* true; the first still short-circuits. | `abort()` → `SIGABRT` | [x] |
| E3 | `bin2hex` | `hex_maxlen <= bin_len * 2U` with `hex_maxlen == bin_len * 2` exactly (buffer big enough for the digits but **not** the NUL). Tested for `bin_len` = 1, 2, 3, 7, 8, 16, 31, 32, 255, 256, 257, 1024 and randomized lengths. | `abort()` → `SIGABRT` | [x] |
| E4 | `bin2hex` | `hex_maxlen <= bin_len * 2U` with `hex_maxlen < bin_len * 2` (short buffer), incl. `hex_maxlen == 0` and `hex_maxlen == bin_len` for the same `bin_len` set as E3. | `abort()` → `SIGABRT` | [x] |
| E5 | `bin2hex` | `hex_maxlen == 0` with `bin_len == 0` — the degenerate case where `bin_len * 2 == 0` and `0 <= 0` holds, so even the empty input is rejected. Exercised with `hex == NULL` / `bin == NULL` too, since the guard runs before any dereference. | `abort()` → `SIGABRT` | [x] |
| E6 | `bin2hex` | Both operands true simultaneously: `bin_len == usize::MAX`, `hex_maxlen == 0`. Confirms the `\|\|` short-circuit is not observable and the result is still `SIGABRT`. | `abort()` → `SIGABRT` | [x] |

## Generic FFI boundary cases (not in the C source's checks, tested anyway)

The C code performs **no** validation of these; the table records the behaviour
the C actually exhibits, and the Rust must match it.

| # | condition | actual C behaviour | Rust must match | [x] |
|---|-----------|--------------------|-----------------|-----|
| G1 | `hex == NULL`, `bin == NULL`, `bin_len == 0`, `hex_maxlen == 0` | guard fires first → `SIGABRT` (no deref) | same | [x] |
| G2 | `hex == NULL`, `bin_len == 0`, `hex_maxlen == 0` | guard fires first → `SIGABRT` | same | [x] |
| G3 | `bin == NULL`, `bin_len == 0`, `hex_maxlen >= 1`, valid `hex` | **accepted**: writes `hex[0] = 0`, returns `hex`. `bin` is never dereferenced because the loop body never runs. | same — Rust must not form a slice from the null `bin` | [x] |
| G4 | zero length: `bin_len == 0`, `hex_maxlen == 1` (smallest accepted) | writes exactly one byte `0`, returns `hex` | same | [x] |
| G5 | `bin_len` one step past the documented valid range, i.e. `9223372036854775807` (E1) vs one step inside, `9223372036854775806` | `…807` → `SIGABRT` (operand 1). `…806` passes operand 1; operand 2 then rejects it unless `hex_maxlen > 0xFFFF_FFFF_FFFF_FFFC`, i.e. only for `hex_maxlen ∈ {MAX-2, MAX-1, MAX}`. For those three, the guard **accepts** and the loop runs off the end of the output buffer. | same, and *which* operand fires must be unobservable-identical. The three accepted cases are observed against an `mmap`ed region followed by a `PROT_NONE` guard page, so both must die from `SIGSEGV` (not `SIGABRT`) at the same point. | [x] |
| G6 | `hex_maxlen == usize::MAX` with small `bin_len` (maximum slack, no overflow in `bin_len*2`) | accepted; only `bin_len*2+1` bytes written | same; bytes past `bin_len*2` untouched | [x] |
| G7 | out-of-range enum values across the FFI boundary | **N/A** — `bin2hex` has no enum, `int`-typed-flag, or mode parameter. Its signature is `(char*, size_t, const uint8_t*, size_t)`: two raw pointers and two `size_t`s, whose full value ranges are already covered by E1–E6/G5. There is no invalid-variant class of input to test. | — | [x] |
| G8 | return-value contract | returns the **same pointer** that was passed as `hex` (never a copy, never NULL) | same pointer value | [x] |

`abort()` terminates the process, so every `SIGABRT` row is exercised in a
`fork()`ed child (`tests/error_paths.rs`), and the parent asserts
`WIFSIGNALED(status) && WTERMSIG(status) == SIGABRT` **for both** the C and the
Rust `.so` — i.e. the same signal, not merely "both failed somehow".

## Divergence found and fixed

`G5` caught one real divergence. The original translation borrowed the output
buffer as a slice:

```rust
let out_len = bin_len * 2 + 1;
let out: &mut [u8] = slice::from_raw_parts_mut(hex.cast::<u8>(), out_len);
```

The C guard accepts `bin_len` up to `9223372036854775806`, for which
`bin_len * 2 + 1` is `18446744073709551613` — larger than `isize::MAX`. A Rust
slice cannot be that long, so `from_raw_parts_mut` tripped its
`debug_assert` and the Rust `.so` **panicked** where the C `.so` proceeded into
the loop and faulted. The same slice-based approach also could not express
`bin == NULL` with `bin_len == 0`, which the C accepts.

Fixed by transliterating the loop with raw pointer arithmetic
(`*hex.wrapping_add(i * 2) = …`), which has neither the `isize::MAX` length
limit, the non-null requirement, nor the address-overflow precondition that
`ptr::add` carries. After the fix the two `.so`s agree in both the release and
the debug profile.

## Test-suite validation (mutation testing)

To confirm the differential tests genuinely bite rather than passing vacuously,
nine deliberate bugs were injected into `src/lib.rs` one at a time and the suite
re-run. All nine were caught:

| injected bug | detected by |
|--------------|-------------|
| swapped nibble emission order | 17/20 valid-path rows |
| uppercase hex digits (`87` → `55`) | 17/20 valid-path rows |
| NUL terminator not written | 20/20 valid-path rows |
| `bin_len` limit off by one | 3/14 error-path rows |
| guard `<=` weakened to `<` | 5/14 error-path rows |
| guard `\|\|` changed to `&&` | 8/14 error-path rows |
| return `NULL` instead of `hex` | 20/20 valid-path rows + G8 |
| adjust mask `~38U` → `~39U` | 16/20 valid-path rows |
| output index `i*2+1` → `i*2` | 17/20 valid-path rows |

One *semantically equivalent* mutation (arithmetic instead of logical shift in
`((n - 10U) >> 8)`) correctly did **not** fail: the subsequent `unsigned char`
cast makes the sign extension unobservable, in C as in Rust.
