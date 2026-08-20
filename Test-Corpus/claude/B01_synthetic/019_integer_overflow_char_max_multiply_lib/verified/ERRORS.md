# ERRORS.md — Phase C error-surface table

Derived by mechanically grepping **all** of `c_src/src/driver.c` and
`c_src/include/driver.h` for every way the code rejects, guards against, or
refuses to act on its input.

## Mechanical grep results

```
$ grep -nE 'return|RETURN_ERROR|assert|NULL|errno|exit\(|abort\(|== *-1|< *0|> *0|MAX|MIN' c_src/src/driver.c
32:    if(line != NULL)            <- null check
46:    data = CHAR_MAX;            <- limits.h max constant
47:    if(data > 0)                <- positivity guard
58:    if(data > 0)                <- positivity guard
69:    data = CHAR_MAX;            <- limits.h max constant
70:    if(data > 0)                <- positivity guard
72:        if (data < (CHAR_MAX/2)) <- range check (the CWE-190 fix)
79:            printLine("data value is too large to perform arithmetic safely.")
91:    if (useGood)                <- mode selection, not an error
```

Notes on what is **absent** (so the table below is complete, not truncated):

* no `return <errcode>` / `return NULL` / `RETURN_ERROR` — **every** function
  in this library returns `void`;
* no `assert`, no `errno` use, no `exit`/`abort`, no error `enum`;
* no allocation, so no allocation-failure path;
* therefore every "error" here is observable **only** as a difference in what
  is written to `stdout` (or as the *absence* of output), never as a return
  value. All rows are asserted on captured `stdout` bytes.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `printLine` | `line == NULL` (`driver.c:32` null check fails) | returns immediately, **writes nothing**, no crash | [x] |
| E2 | `printLine` | `line` non-NULL but pointing at a bare NUL byte (`""`) — passes the null check, degenerate content | writes exactly `"\n"` (1 byte) | [x] |
| E3 | `printLine` | `line` contains `printf` conversion specifiers (`"%s"`, `"%n"`, `"%d"`, `"%%"`) — potential format-string mis-use | passed as the *argument* of `"%s\n"`, so emitted **literally** + `"\n"`; no format-string interpretation | [x] |
| E4 | `printLine` | `line` contains bytes ≥ 0x80 / invalid UTF-8 (`\xff\xfe\x80`) | copied byte-for-byte + `"\n"`; C `%s` is byte-oriented, no encoding validation | [x] |
| E5 | `printLine` | `line` contains an embedded `\n` | copied verbatim + trailing `"\n"` (no de-duplication) | [x] |
| E6 | `printLine` | very long `line` (64 KiB, > `BUFSIZ`) — stdio buffer-overflow-of-buffer path | full string + `"\n"`, correct across stdio buffer refills | [x] |
| E7 | `printHexCharLine` | `charHex` negative (`-1`, `-2`, `-128` = `CHAR_MIN`) — value out of `%02x`'s *unsigned* domain | `char`→`int` promotion **sign-extends**, `%02x` reinterprets as `unsigned`, so 8 hex digits: `ffffffff`, `fffffffe`, `ffffff80` (the `02` width never truncates) | [x] |
| E8 | `printHexCharLine` | `charHex` = `0` (degenerate/empty value) | `"00\n"` — zero-padded to width 2, not `"0\n"` | [x] |
| E9 | `printHexCharLine` | caller passes an out-of-`char`-range `int` across the FFI boundary (mismatched prototype, e.g. `0x12345678`, `256`, `-1000`) | callee narrows to the low byte and sign-extends it (`movsbl`); upper 24 bits are ignored. **This row found a real bug — see "Divergence found" below.** | [x] |
| E10 | `bad` | *unreachable* false branch of `if(data > 0)` (`driver.c:47`): `data` is hard-coded to `CHAR_MAX` = 127, so the guard can never fail | guard always taken; `bad()` **always** prints `"fffffffe\n"` and never nothing | [x] |
| E11 | `bad` | the CWE-190 overflow itself: `(char)(CHAR_MAX * 2)` = `(char)254` — signed-char overflow on truncation | C truncates to `-2`; result is `"fffffffe\n"`. The Rust MUST NOT panic on overflow nor saturate | [x] |
| E12 | `goodG2B` (via `good`/`driver(≠0)`) | *unreachable* false branch of `if(data > 0)` (`driver.c:58`): `data` hard-coded to `2` | guard always taken; always prints `"04\n"` | [x] |
| E13 | `goodB2G` (via `good`/`driver(≠0)`) | *unreachable* false branch of `if(data > 0)` (`driver.c:70`): `data` hard-coded to `CHAR_MAX` | guard always taken | [x] |
| E14 | `goodB2G` (via `good`/`driver(≠0)`) | **the range-check rejection** `if (data < (CHAR_MAX/2))` (`driver.c:72`) fails: `127 < 63` is false | takes the `else`, rejects the arithmetic and prints the diagnostic `"data value is too large to perform arithmetic safely.\n"`; **no** hex line is emitted | [x] |
| E15 | `goodB2G` | dead store `data = ' '` immediately overwritten by `data = CHAR_MAX` (`driver.c:68-69`) | the `' '` (0x20) value must have **no** observable effect — output must never be `"40\n"` (which is what `' ' * 2` would print) | [x] |
| E16 | `driver` | `useGood == 0` (the "false" value) | dispatches to `bad()` → `"fffffffe\n"` | [x] |
| E17 | `driver` | `useGood` nonzero but with a **zero low byte** (`256`, `0x10000`, `0xFFFFFF00`) — truthiness must be tested on the full `int`, not a narrowed byte | dispatches to `good()`; a Rust bug testing `as u8 != 0` would wrongly pick `bad()` | [x] |
| E18 | `driver` | `useGood` at the extremes of `int`: `INT_MIN` (`-2147483648`), `INT_MAX`, `-1` — one step past / at the documented range ends | all are nonzero ⇒ `good()`; `INT_MIN` must not be mistaken for false via `abs`/negation | [x] |
| E19 | `driver` | `useGood` given an out-of-range "enum-like" `int` (no valid variant: `2`, `3`, `-7`, `0x7f`, random 32-bit) — C `int` params accept any bit pattern | every nonzero value is treated as `good()`; there is no validation and no rejection | [x] |
| E20 | all 5 exports | repeated / interleaved invocation (no re-entrancy guard, no init function, no state) | each call is independent and idempotent; output is the exact concatenation of the individual calls | [x] |

All 20 rows are covered by `tests/phase_c_errors.rs`, whose test names carry
their `E` number (`e1_...`, `e9_...`, `e17_...`), plus four `generic_*` tests for
the boundaries every C API has. All 20 rows pass against both `.so` files, in
both the dev and the release profile. See the run log in `CONFIGS.md`.

## Divergence found and fixed (row E9)

This is the one place where the Rust diverged from the C, and it was invisible
in the default (unoptimised) test configuration.

**Symptom.** `e9_print_hex_char_line_out_of_range_int_across_ffi` and
`generic_one_step_past_every_documented_range` failed, but *only* when the Rust
crate was built with `--release`:

```
DIVERGENCE [E9 printHexCharLine(widened 0x12345678)]
DIVERGENCE [generic printHexCharLine(widened -129)]
```

**Root cause.** `printHexCharLine` takes a `char`. Under the SysV AMD64 psABI
the upper 24 bits of a narrow integer argument register are *unspecified*, so
the two sides disagreed about who is responsible for narrowing:

| build | instruction that consumes the argument | behaviour |
|-------|----------------------------------------|-----------|
| C, `cmake` default (`-O0`) | `mov %edi,%eax; mov %al,-0x4(%rbp); movsbl -0x4(%rbp),%eax` | narrows defensively |
| C, `gcc -O2` | `movsbl %dil,%esi` | narrows defensively |
| Rust `dev`, param typed `c_char` | `mov %dil,%al; ... movsbl %al,%esi` | narrows (accidentally) |
| Rust `release`, param typed `c_char` | `mov %edi,%esi` | **does not narrow** |

Declaring the Rust parameter as `c_char` makes rustc attach LLVM's `signext`
attribute, which is a promise that the *caller* extended the byte; an optimised
build is then free to forward the whole 32-bit register. gcc makes no such
assumption about its callers and narrows at **every** optimisation level.

**Fix** (`src/lib.rs`): declare the parameter as `c_int` and narrow explicitly,
reproducing the C's own instruction sequence.

```rust
pub unsafe extern "C" fn printHexCharLine(charHex: c_int) {
    let charHex = charHex as u8 as c_char;   // == gcc's movsbl
    printf(b"%02x\n\0".as_ptr() as *const c_char, charHex as c_int);
}
```

After the fix the optimised Rust emits `movsbl %dil,%esi` — the *identical*
instruction to `gcc -O2`. This is byte-for-byte identical for every
ABI-conforming caller, and now also identical for non-conforming ones.

`tests/phase_d_symbols.rs::optimised_c_build_agrees_with_rust_on_the_whole_surface`
compiles `c_src/src/driver.c` a second time at `-O2` (writing only into
`target/`, never into `c_src/`) and re-runs the comparison, so this bug class is
covered against both C optimisation levels from now on.

## Negative control (is this suite actually able to fail?)

Passing tests only mean something if the suite rejects a broken translation, so
`mutation_check.sh` injects 15 deliberate bugs into `src/lib.rs` and requires
the suite to fail for each, then confirms the restored tree passes:

| # | injected bug | caught by (example) |
|---|--------------|---------------------|
| M1 | `printHexCharLine`: sign extension dropped | `c1`, `c12`, `e7` |
| M2 | arithmetic saturates instead of wrapping | `c12`, `e11` |
| M3 | `driver`: truthiness tested on the low byte only | `c11`, `e17` |
| M4 | `printLine`: NULL treated like `""` | `c15` (dev) / SIGSEGV in `puts(NULL)` (release) |
| M5 | `goodB2G`: range check inverted | `c11`, `e14` |
| M6 | `goodG2B`: `data = 3` instead of `2` | `c11`, `e12` |
| M7 | `goodB2G`: dead store not overwritten | `c11`, `e15` |
| M8 | `driver`: branches swapped | `c11`, `e16` |
| M9 | `printLine`: trailing newline dropped | `c11`, `e2` |
| M10 | `printHexCharLine`: `%x` instead of `%02x` | `c11`, `e8` |
| M11 | `good`: `#[no_mangle]` export removed | symbol parity + dlsym resolution |
| M12 | `good`: `goodG2B`/`goodB2G` order swapped | `c9`, `c14` |
| M13 | `printHexCharLine`: defensive narrowing removed (the E9 bug) | `e9` |
| M14 | `bad`: positivity guard flipped | `c12`, `e10` |
| M15 | `printLine`: payload used as the format string | `c11`, `e3` |

`RESULT: all 15 mutations were caught by the differential suite.` in **both**
the dev and the release profile, and `pristine: PASS` afterwards.
