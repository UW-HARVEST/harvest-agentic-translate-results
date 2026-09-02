# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived mechanically from the C source, not from documentation or assumption.

## Mechanical derivation

Grep over `c_src/src` and `c_src/include` for every rejection construct:

```sh
grep -rnE 'return +-|return +NULL|RETURN_ERROR|assert|errno|goto|if *\(|switch|#if|enum|ERROR|E[A-Z]+' \
     c_src/src c_src/include
# (no matches)
```

Statement-level inventory of `c_src/src/lib.c`: 5 statements, all of them
unconditional assignments/returns.

Findings:

* error-return macros (`RETURN_ERROR`, …): **0**
* `return -1` / `return NULL` / negative or sentinel error returns: **0**
* error enums or status codes: **0** — the only return type is `uint32_t`, and
  every one of the 2^32 possible return values is a legitimate result
* `assert` / `abort` / `errno` writes: **0**
* explicit range checks, null checks, size checks: **0**
* min/max constants: **0** (the four hex literals `0xAAAA 0x5555 0xCCCC 0x3333
  0xF0F0 0x0F0F 0xFF00 0x00FF` are bit masks, not bounds)
* pointer parameters anywhere in the API: **0** (`rev16` takes a `uint32_t` by
  value and returns a `uint32_t` by value)
* enum parameters anywhere in the API: **0**
* branches (`if` / `switch` / ternary / `#ifdef`): **0**

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | `rev16`  | *none — the function is total*              | n/a               |

The table is intentionally empty of rejection rows: `rev16` is a pure, total
function over `uint32_t`. There is no input it rejects, no code path that can
fail, and no out-of-band error channel (no return sentinel, no `errno`, no
out-parameter, no pointer to dereference).

## Generic boundary rows tested anyway (Phase C)

Because "no rejection path exists" is itself a claim that must be verified
differentially — the Rust must also *not* reject, panic, or abort where the C
computes a value — the following boundary and adversarial inputs are covered by
`tests/differential.rs` (`phase_c_*` tests). Each asserts C and Rust return the
**same 32-bit value**, and that neither aborts.

| # | function | boundary input | expected behaviour |
|---|----------|----------------|--------------------|
| C1 | `rev16` | `0x0000_0000` (zero / minimum) | returns `0x0000_0000`, no error |
| C2 | `rev16` | `0xFFFF_FFFF` (`UINT32_MAX`, maximum) | returns `0x0000_FFFF`, no error |
| C3 | `rev16` | `0x0000_FFFF` (largest value whose bits all survive the 16-bit masks) | returns `0x0000_FFFF`, no error |
| C4 | `rev16` | `0x0001_0000` (one step past the 16-bit "documented range") | returns `0x0000_0000`, no error |
| C5 | `rev16` | `0x8000_0000` (top bit only — sign bit if misread as signed `int`) | returns `0x0000_0000`, no error |
| C6 | `rev16` | `0x7FFF_FFFF` (`INT32_MAX`) | returns `0x0000_FFFF`, no error |
| C7 | `rev16` | `0x8000_0001` / `0xFFFF_0001` (negative when misread as signed) | upper half discarded; equals `rev16(0x1)` = `0x8000` |
| C8 | `rev16` | out-of-range "enum-like" ints: `-1`, `-2147483648`, `2147483647`, `0xDEAD_BEEF` passed through the FFI boundary as `c_uint` | C accepts any bit pattern; Rust must produce the identical value and must not panic |
| C9 | `rev16` | every value of the form `1u32 << k` for `k = 0..=31` (walking one, including the 16 bits that are silently dropped) | `k < 16` → `1 << (15-k)`; `k >= 16` → `0` |
| C10 | `rev16` | `0xFFFF_0000` (only the discarded half set — "oversized length" analogue) | returns `0x0000_0000`, no error |
| C11 | `rev16` | repeated invocation with the same and with alternating inputs (no hidden state / no `errno`-style stickiness) | results identical to single invocation, in both objects |

Rows C1–C11 are the complete Phase C checklist. All are checked off in
`CONFIGS.md`-style form at the bottom of `tests/differential.rs`.
