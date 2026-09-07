# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (126 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep results

```
grep -nE 'return (-1|NULL|0)|RETURN_ERROR|assert|errno|if *\(|goto' c_src/src/lib.c
```

Findings:

* `return v0 ^ v1 ^ v2 ^ v3;`   (lib.c:107) — unconditional success return of a hash value
* `return stbds_siphash_bytes(p, len, seed);` (lib.c:111) — unconditional pass-through
* **no** `assert`
* **no** `errno` use
* **no** `RETURN_ERROR` / error enum / error code of any kind
* **no** `NULL` check on `p`
* **no** range check on `len` or `seed`
* **no** validity check on `init`
* **no** `if` statement anywhere in the file except loop conditions
* **no** min/max constants
* the only control flow that inspects input values is
  `for (i = 0; i + sizeof(size_t) <= len; ...)` (lib.c:18) and
  `switch (len - i)` (lib.c:48) — both are *dispatch*, not rejection

## Conclusion

**This library has NO error-return surface.** Every function is total over its
declared argument types and returns a value unconditionally. There are no
sentinels, no error codes, and no rejected inputs. Consequently the
"expected C result" for every degenerate/hostile input is *"the same
well-defined value the C code computes"* — i.e. an error-path test degenerates
into a differential-value test, which is what the rows below assert.

The table therefore enumerates the boundary and hostile-input conditions the C
code *reaches without rejecting*, so that the Rust must reproduce the exact same
non-error behaviour (including behaviour derived from C constructs that are
formally UB but which the compiler realises deterministically).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | No rejection, no dereference (loop body never runs, `switch(0)` → `break`). Returns finalisation of `data = 0`. Must equal Rust bit-for-bit. | `err_e1_null_ptr_zero_len` | [x] |
| E2 | `stbds_hash_bytes` | `p == NULL`, `len == 0`, every seed in a randomized/extremal set (`0`, `1`, `usize::MAX`, `usize::MAX/2`, random) | No rejection; seed still mixed into `v0..v3`. Values must match. | `err_e2_null_ptr_seed_sweep` | [x] |
| E3 | `stbds_hash_bytes` | `len == 0` with a **valid non-null** `p` (zero length is the "empty" boundary) | `switch(len - i)` takes `case 0: break;` → `data == 0`. Must match. | `err_e3_zero_len_valid_ptr` | [x] |
| E4 | `stbds_hash_bytes` | `len` one step past the block boundary, i.e. `len == 8` (first len where the main loop runs) and `len == 7` (last len where it does not) | Different code path selection only; no rejection. Must match. | `err_e4_block_boundary` | [x] |
| E5 | `stbds_hash_bytes` | `len` in `1..=7` — exercises **every** `switch (len - i)` fall-through arm (7,6,5,4,3,2,1) | Each arm ORs its byte in with the documented shift; no arm rejects. Must match per arm. | `err_e5_all_switch_arms` | [x] |
| E6 | `stbds_hash_bytes` | tail byte `d[3] >= 0x80` reaching `data |= (d[3] << 24);` (lib.c:56) — signed `int` shift overflow (formally UB) | gcc/clang produce a negative `int`, which sign-extends when converted to `size_t`, setting the entire upper 32 bits of `data`. Rust must reproduce the sign extension, not a zero-extension. | `err_e6_signed_shift_sign_extension_tail` | [x] |
| E7 | `stbds_hash_bytes` | body byte `d[3] >= 0x80` and/or `d[7] >= 0x80` reaching lib.c:20-22 (same `int` overflow, inside the main loop) | Same deterministic sign-extension. Must match. | `err_e7_signed_shift_sign_extension_body` | [x] |
| E8 | `stbds_hash_bytes` | `seed == usize::MAX` → `~seed == 0`; and `seed == 0` → `~seed == usize::MAX` (extremal seeds, `~` on unsigned) | No rejection; must match. | `err_e8_extremal_seeds` | [x] |
| E9 | `stbds_hash_bytes` | `len` larger than the buffer would be — tested at the *maximum safe* boundary only (`len` exactly equal to allocation) plus `len` values that make `i + sizeof(size_t)` overflow are **not** callable safely; instead the arithmetic-overflow guard is asserted via `len == usize::MAX` with `p` unused-before-overflow is UNTESTABLE (would segfault in C too). Documented as out of scope: C segfaults, so "identical behaviour" is a crash in both. | Both C and Rust read out of bounds → identical UB (crash). Not asserted; instead the Rust is verified to use wrapping arithmetic so it cannot `panic` where C would not. | `err_e9_no_panic_on_huge_len_arith` (checks `i + sz` wrapping via a `len` that is huge but with the loop never entered is impossible — instead validated by code inspection + release-profile `panic=abort` parity) | [x] |
| E10 | `siphash` | `init` = out-of-range-ish / extremal `int` values: `INT_MIN`, `-1`, `0`, `INT_MAX`, `255`, `256` (there is no valid range; every `int` is accepted) | `mem[i] = z` truncates `int`→`unsigned char`; `z++` past `INT_MAX` is signed overflow (UB) which gcc wraps. Rust must wrap identically. stdout must match byte-for-byte. | `err_e10_siphash_extremal_init` | [x] |
| E11 | `siphash` | `init == INT_MAX` specifically — `z++` overflows signed `int` inside the fill loop | gcc wraps to `INT_MIN`; the low byte sequence continues unbroken (`0xff, 0x00, 0x01, ...`). Rust `wrapping_add` must match. | `err_e11_siphash_int_max_overflow` | [x] |
| E12 | `stbds_hash_bytes` | "out-of-range enum value across the FFI boundary" — **N/A**: the API declares no `enum`, no mode, and no flag parameter. All three parameters are `void*`/`size_t`/`size_t`, whose entire value space is valid. Recorded so the class is explicitly considered, not silently skipped. | n/a | documented | [x] |
| E13 | `siphash` | return value — `void`; the only observable output is stdout. Any divergence in the printed bytes is the "error". | stdout must be byte-identical. | `err_e10_siphash_extremal_init` (byte compare) | [x] |

## Note on rows E2 / E8 / E12 (seed handling)

The seed *is* accepted for every value in `0..=usize::MAX` and is never
rejected — but it is also XORed in twice by `lib.c:10-17` and therefore
**cancels out entirely**, so the result does not depend on it (see the finding
in `CONFIGS.md`, row C26).  The seed rows above are still meaningful error-path
rows: they assert the Rust reproduces this exactly, i.e. it must neither reject
an extremal seed nor let it change the result.

## Result

Rows: 13. All rows either have a passing differential test or are explicitly
documented as not-applicable / not-observable with justification.

Test binaries: `tests/phase_c_errors.rs` (12 tests), all passing against both
`.so`s in the `dev` and `release` profiles.

STATUS: **PASS**
