# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from `c_src/src/driver.c`, `c_src/include/driver.h` and
`c_src/CMakeLists.txt`.

## Axes the C code actually distinguishes

**Runtime options / modes / flags:** none. Grepping the public header and the
source for `if`, `switch`, `#ifdef`, `#if`, and any setter/global yields nothing
— there is no configuration state, no global variable, no init/teardown, and no
flag argument anywhere. `driver.h` declares a single function.

```
$ grep -nE '#if|#ifdef|switch|if *\(|static|extern|global' c_src/src/driver.c
(nothing outside the include guard / license header)
```

**Public entry points (full set, including the lowest level):**

| entry point | declared in header? | exported from `.so`? | level |
|---|---|---|---|
| `printHexCharLine(char)` | no | **yes** (`T`) | lowest level — the formatting primitive |
| `driver(char)` | yes | yes (`T`) | one-shot wrapper: `printHexCharLine(data + 1)` |

Both are exercised **directly** below; `driver` is not used as a proxy for
`printHexCharLine`.

**Input shapes the code / ABI special-cases** (the only argument is one `char`):

* `A1` element type / width: exactly one 8-bit `char`; on the x86-64 Linux
  target `char` is **signed**, so the promotion in the `printf` call is a *sign*
  extension. This is the single most behaviour-defining axis.
* `A2` sign of the value: non-negative (`0x00`–`0x7f`) → 2 hex digits, vs.
  negative (`0x80`–`0xff`) → 8 hex digits after sign extension.
* `A3` magnitude vs. the `%02x` zero-pad width: value `< 0x10` (1 significant
  digit → padded to `00`..`0f`) vs. `>= 0x10` (no padding).
* `A4` boundary values: `0x00`, `0x0f`/`0x10` (pad boundary), `0x7f`/`0x80`
  (sign boundary), `0xfe`, `0xff`.
* `A5` for `driver` only: whether `data + 1` overflows `char` (`data == 0x7f`),
  crosses the sign boundary downward (`data == 0xff` → `0x00`), or crosses the
  pad boundary (`data == 0x0f` → `0x10`).
* `A6` ABI over-wide argument: the caller pushes a full register; passing an
  `int` outside `[-128,127]` (`0x100`, `0x1ff`, `-1000`, `INT_MIN`, `INT_MAX`)
  exercises whether the callee truncates to 8 bits identically in C and Rust.
* `A7` call count / sequencing: one call vs. many calls vs. **interleaved** C and
  Rust calls sharing the process's `stdout` — the only observable state in the
  whole library is stdio buffering, so ordering/flush behaviour is an axis.
* `A8` output destination shape: `stdout` connected to a regular file / pipe
  (fully buffered, the mode used by the harness) — the format string ends in
  `\n` but `printf` does not force a flush when `stdout` is not a tty, so the
  test must `fflush` and compare captured bytes.

Cross-product pruned to the combinations the code actually distinguishes:

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printHexCharLine` | `0x00` — zero, non-negative, needs zero-padding (A2+,A3<,A4) | [x] |
| 2 | `printHexCharLine` | randomized `0x01`–`0x0f` — non-negative, 1 significant digit, zero-padded | [x] |
| 3 | `printHexCharLine` | `0x10` — pad boundary, first value needing no padding | [x] |
| 4 | `printHexCharLine` | randomized `0x11`–`0x7e` — non-negative, 2 digits, no padding | [x] |
| 5 | `printHexCharLine` | `0x7f` — `CHAR_MAX`, last non-negative | [x] |
| 6 | `printHexCharLine` | `0x80` — `CHAR_MIN`, first negative, sign-extends to `ffffff80` | [x] |
| 7 | `printHexCharLine` | randomized `0x81`–`0xfe` — negative, sign-extended 8-digit output | [x] |
| 8 | `printHexCharLine` | `0xff` — `-1`, sign-extends to `ffffffff` | [x] |
| 9 | `printHexCharLine` | **exhaustive** sweep of all 256 `char` bit patterns, one call per value | [x] |
| 10 | `driver` | `0x00` — no wrap, result `0x01`, zero-padded | [x] |
| 11 | `driver` | randomized `0x01`–`0x0e` — result stays in `0x02`–`0x0f`, zero-padded | [x] |
| 12 | `driver` | `0x0f` — result crosses the pad boundary to `0x10` | [x] |
| 13 | `driver` | randomized `0x10`–`0x7d` — result non-negative, 2 digits | [x] |
| 14 | `driver` | `0x7e` — result `0x7f` = `CHAR_MAX`, still non-negative | [x] |
| 15 | `driver` | `0x7f` — **signed overflow of `data + 1`**, wraps to `-128`, prints `ffffff80` | [x] |
| 16 | `driver` | `0x80` — most negative input, result `0x81`, prints `ffffff81` | [x] |
| 17 | `driver` | randomized `0x81`–`0xfd` — negative in, negative out, 8-digit output | [x] |
| 18 | `driver` | `0xfe` — result `-1`, prints `ffffffff` | [x] |
| 19 | `driver` | `0xff` — result crosses the sign boundary downward to `0x00`, prints `00` | [x] |
| 20 | `driver` | **exhaustive** sweep of all 256 `char` bit patterns, one call per value | [x] |
| 21 | `printHexCharLine` | A6: over-wide `int` argument (`0x100`, `0x1ff`, `-1000`, `INT_MIN`, `INT_MAX`, randomized `i32`) called through an `extern "C" fn(c_int)` view of the symbol | [x] |
| 22 | `driver` | A6: over-wide `int` argument, same set, through an `fn(c_int)` view | [x] |
| 23 | both, interleaved | A7: long randomized sequence alternating `driver` and `printHexCharLine` on the same library, comparing the whole concatenated stdout stream in one capture | [x] |
| 24 | both, interleaved | A7+A8: C and Rust calls interleaved 1:1 within a single captured stdout region, asserting the two streams are byte-identical line-for-line and that neither library flushes differently | [x] |
| 25 | `driver` then `printHexCharLine` | A7: composed pipeline — `driver(x)` must equal `printHexCharLine(x+1)` in **both** libraries (cross-library composition check, randomized) | [x] |
| 26 | both | A8: repeated calls with no intervening flush (buffered `stdout` to a pipe), flushed once at the end — verifies identical byte stream, not just identical per-call text | [x] |

## Divergence found and fixed by this table

**Row 21** (`printHexCharLine` with an over-wide `int`) failed — but **only in the
`--release` profile**:

```
printHexCharLine(128i32 / 0x00000080): C printed "ffffff80\n" but Rust printed "80\n"
```

Cause: GCC re-narrows the incoming argument register in the prologue —

```
printHexCharLine:
    mov    %edi,%eax
    mov    %al,-0x4(%rbp)      ; keep only the low 8 bits
    movsbl -0x4(%rbp),%eax     ; sign-extend them back to int
```

— so the C callee ignores bits 8..31. Rust's `extern "C" fn(c_char)` lowers the
parameter with LLVM's `signext i8` attribute, which lets an optimised build
*assume* the caller already sign-extended and forward `%edi` unchanged. At `-O0`
the truncation happened incidentally, which is why the debug profile passed and
only the release profile exposed the bug.

Fix (`translation/src/lib.rs`): both exports now take `c_int` and narrow
explicitly with `as u8 as c_char`, reproducing GCC's `mov %al` + `movsbl`. The
release codegen is now byte-level, matching the C:

```
driver:            inc %dil ; movsbl %dil,%esi ; ... jmp printf
printHexCharLine:  movsbl %dil,%esi ; ... jmp printf
```

All 26 rows pass in **both** profiles after the fix; reverting the fix makes
row 21 fail again in release (verified).
