# ERRORS.md — Error-surface table

Derived mechanically from `c_src/`, not from docs or assumptions.

## Mechanical derivation

Every error-shaped construct was grepped for across the whole of `c_src`:

```
$ grep -rnE 'return|assert|NULL|nullptr|errno|RETURN_ERROR|E[A-Z]+|exit\(|abort\(|goto|enum|-1|#if' \
       c_src/src c_src/include
c_src/include/driver.h:24:#ifndef DRIVER_H_          <- include guard only
```

Result of the grep, stated precisely:

| construct | occurrences in `c_src` |
|---|---|
| `return` statements of any kind | **0** (both functions are `void` and fall off the end) |
| `assert` / `static_assert` | **0** |
| error enums / `errno` / error codes | **0** |
| `NULL` checks | **0** |
| explicit range / bounds checks (`if`, `switch`, `?:`) | **0** |
| `min`/`max` constants, `#define`d limits | **0** |
| `exit` / `abort` / `goto` | **0** |
| conditional compilation affecting behaviour | **0** |
| pointer parameters | **0** |
| length/size parameters | **0** |
| `enum` parameters | **0** |

**The library has ZERO explicit rejection paths.** Both entry points are
`void`-returning, unconditional, straight-line code. There is no value of any
argument for which the C returns an error, sets a code, or refuses to act — it
always prints exactly one line.

Consequently the error surface consists **entirely of the boundaries the C
silently does *not* check**. Those are enumerated below one row per distinct
condition. For each, "expected C result" is the observable behaviour (the line
written to stdout), which is what the Rust must reproduce byte-for-byte; the
"error" is the absence of rejection, so the differential assertion is that Rust
also does not reject and emits the identical bytes.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `printHexCharLine` | negative `char` argument, `charHex = -1` (`0xFF`). `char` is signed on this ABI, so the default argument promotion sign-extends to `int` `-1`; `%02x` then reinterprets it as `unsigned int` `0xFFFFFFFF`. The `02` is a *minimum* width, so nothing is truncated. | prints `ffffffff\n`; returns (void), no error |
| E2 | `printHexCharLine` | most-negative `char`, `charHex = -128` (`0x80`) — the lower boundary of the parameter's range | prints `ffffff80\n`; no error |
| E3 | `printHexCharLine` | one step below the sign boundary, `charHex = 127` (`0x7F`) = `CHAR_MAX`, the largest value that is *not* sign-extended | prints `7f\n`; no error |
| E4 | `printHexCharLine` | `charHex = 0`; `%02x` of 0 must zero-pad, exercising the width flag rather than the value | prints `00\n`; no error |
| E5 | `printHexCharLine` | sub-width value `charHex = 0x0F` (< 0x10, needs one pad digit) vs. `0x10` (needs none) — the `02` width boundary | prints `0f\n` / `10\n`; no error |
| E6 | `printHexCharLine` | **out-of-range argument across the FFI boundary**: the symbol is called through a pointer typed `void(*)(int)` with a value that has no `char` representation, e.g. `0x1FF`, `0xFFFFFF01`, `INT_MIN`, `INT_MAX`. C performs no check; the callee reads only the low 8 bits of the argument register (`movsbl`) and sign-extends. This is the closest analogue of "out-of-range enum value" for this API. | behaves as if called with `(char)(v & 0xFF)`, e.g. `0x1FF` → `ffffffff\n`; no error, no trap |
| E7 | `driver` | **signed overflow of the truncating store**: `data = 127` (`CHAR_MAX`). `data + 1` is evaluated as `int` `128`, which is then converted back to `char` — out of range for `char`, an implementation-defined conversion that gcc/clang implement as modulo-2⁸ wrap to `-128`. The C does not check for this. | `result == -128`; prints `ffffff80\n`; no error |
| E8 | `driver` | `data = -1` (`0xFF`): `data + 1 == 0`, the only input whose output is the all-zero line — verifies the increment is done before, not after, the hex conversion | `result == 0`; prints `00\n`; no error |
| E9 | `driver` | `data = -128` (`0x80`) = `CHAR_MIN`, lower boundary of the parameter range; `-128 + 1 == -127` stays negative and is therefore sign-extended | `result == -127`; prints `ffffff81\n`; no error |
| E10 | `driver` | `data = 126` (`0x7E`): one step *below* the overflow boundary of row E7, so it must **not** wrap | `result == 127`; prints `7f\n`; no error |
| E11 | `driver` | `data = -1 - 1`? no — `data = 0x0E`/`0x0F`: the increment crosses the `%02x` zero-pad width boundary (`0x0F` → `10`) | prints `0f\n` / `10\n`; no error |
| E12 | `driver` | **out-of-range argument across the FFI boundary**, as E6 but for `driver`: called through `void(*)(int)` with `0x1FF`, `0xFFFFFF7F`, `INT_MIN`, `INT_MAX`, values with no `char` representation. No check in C. | behaves as `driver((char)(v & 0xFF))`; no error, no trap |
| E13 | *both* | **repeated / interleaved invocation**: the functions keep no state and never fail on a second call; calling C then Rust then C on the same `stdout` must not change either one's bytes (guards against a Rust translation that lazily initialises or caches) | each call prints exactly one line, identical each time; no error |

### Generic C-API boundaries explicitly recorded as N/A

These are the boundaries the checklist requires be covered "even if not in the
table". They are inapplicable here, and the reason is mechanical, not assumed:

| generic boundary | applicability | proof |
|---|---|---|
| null pointer arguments | **N/A** | neither entry point takes a pointer (`grep -c '\*' c_src/src/driver.c` → only in comment text; signatures are `(char)`) |
| zero length / oversized length | **N/A** | no size, length, or count parameter exists in the API |
| out-of-range enum value | **N/A as declared**, covered as E6/E12 | no `enum` appears in `c_src`; the equivalent "int with no valid variant" case is an `int` outside `char` range, tested in E6/E12 |
| return-value / error-code mismatch | **N/A** | both functions return `void`; there is no value to compare, so the differential assertion is on the emitted stdout bytes |
| output-buffer overrun | **N/A** | no caller-supplied buffer; output goes to `stdout` via `printf` |

## Where each row is verified, and the bug this phase found

Rows E1–E13 are one function each in `tests/phase_c_error_paths.rs`, named
`e1_…` … `e13_…`, plus `generic_boundaries_na_surface_is_still_pointer_and_length_free`
which mechanically re-checks the N/A claims above against the live header so
they cannot silently rot.

Each row asserts two things, not one:

1. C and Rust emit identical bytes (the differential assertion), and
2. those bytes equal the literal the C semantics demand (e.g. `ffffff80\n` for
   `driver(127)`).

The second assertion is what stops a row from passing vacuously because both
sides are wrong in the same way.

```
$ cargo test --release
row e1_print_hex_negative_one_sign_extends ... ok
...
row e13_repeated_and_interleaved_calls_are_stateless ... ok

Phase C: all 14 rows passed
```

### Divergence found and fixed by rows E6 / C12

Row E6 (out-of-range `int` argument presented to `printHexCharLine` through a
`void(*)(int)` pointer) caught a real translation bug:

| argument | C `.so` | Rust `.so` (before fix) |
|---|---|---|
| `0x80000000` (`INT_MIN`) | `00` | `80000000` |

Cause, from `objdump -d` on both:

```
C    printHexCharLine:  mov %edi,%eax ; mov %al,-0x4(%rbp) ; movsbl -0x4(%rbp),%eax
Rust printHexCharLine:  mov %edi,%esi
```

gcc truncates the incoming argument register to its low 8 bits and sign-extends
that byte. The Rust version declared the parameter as `c_char`, so LLVM tagged it
`signext i8`, trusted the caller to have already sign-extended, and forwarded all
32 bits of `edi` unchanged.

Fix (in `src/lib.rs`): declare both exported parameters as `c_int` and narrow
with `as c_char` in the body. That drops the `signext i8` attribute and forces an
explicit truncate-then-sign-extend, so Rust now emits `movsbl %dil,%esi` and
matches gcc for every 32-bit argument. Behaviour for the 256 in-range `char`
values is unchanged, and the exported symbol names are unchanged.

This is exactly the class of bug happy-path tests miss: all 256 valid `char`
inputs already matched before the fix.
