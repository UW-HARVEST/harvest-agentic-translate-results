# CONFIGS.md — Phase A: the configuration-surface table

## Build-time configurations (feature combinations)

| source | configuration knobs found |
|--------|---------------------------|
| `Cargo.toml` | **no `[features]` table at all** ⇒ the only combination is the empty one |
| `c_src/CMakeLists.txt` | `cmake_minimum_required` / `project` / `add_executable` only — no `option()`, no `add_definitions`, no `target_compile_definitions`, no `if()` |
| `c_src/src/main.c` | **zero** `#if` / `#ifdef` / `#ifndef` directives |

So the complete enumeration of valid feature combinations is exactly one:

| # | combination | `cargo check` command |
|---|-------------|-----------------------|
| F1 | *(no features)* | `cargo check --no-default-features` |

For completeness the suite is nevertheless run under both **`cargo test`
(dev profile, `panic=unwind`)** and **`cargo test --release` (release profile,
`panic=abort`, LTO-less opt-level 3)**, because those really do compile
different code — in particular the `core::arch::asm!` block and the
saturating-arithmetic paths — and `--no-default-features` / `--all-features`
are also both exercised (they are no-ops here, which the script asserts).

## Runtime configurations

The public API is **one parameterless function**.  There are no flags, no
option structs, no modes, and neither `argv` nor the environment is read.  Everything the
C code branches on is therefore either the **shape of the stdin byte stream**
(as interpreted by glibc's `"%d %d"`), the **arithmetic class of `(x, y)`** (as
interpreted by `idiv`), or the **state of the std file descriptors and signal
dispositions**.  Those are the axes enumerated below.

### Axes derived from the source

* **A — leading/intervening/trailing whitespace** (glibc eats `isspace()` before
  every conversion; the literal `' '` in `"%d %d"` is redundant):
  none / single space / multiple / `\t` / `\n` / `\v` / `\f` / `\r` / mixed runs.
* **B/C — sign of each operand**: absent / `+` / `-` (and `-0`, `+0`).
* **D — magnitude class of each operand**: 0 digits (invalid) / 1 digit /
  ordinary / leading zeros / `INT_MAX` / `INT_MIN` / just past `int` /
  `LONG_MAX` / `LONG_MIN` / just past `long` / thousands of digits.
* **E — token count**: 0 / 1 / 2 / more than 2 (extras are never read).
* **F — what terminates each number**: EOF / whitespace / non-digit / NUL /
  byte ≥ 0x80.
* **G — base-prefix shapes** glibc's prefix probe special-cases: `0`, `0x…`,
  `0X…`, `010`.
* **H — arithmetic class of `(x, y)` for `idiv`**: `y == 0` / `INT_MIN ÷ -1` /
  exact / inexact / all four sign quadrants / `|x| < |y|` / `x == 0` /
  `y == ±1` / `x == y`.
* **I — stdin kind**: pipe / regular file / `/dev/null` / closed fd 0 /
  pre-filled writer-closed pipe.
* **J — stdout kind**: pipe / regular file / `/dev/null` / `/dev/full` /
  closed fd 1 / pipe with the reader closed.
* **K — inherited signal dispositions** (the only state that survives `exec`):
  `SIGPIPE` `SIG_DFL`/`SIG_IGN`, `SIGFPE` `SIG_DFL`/`SIG_IGN`/blocked.
* **L — entry point**: the `driver_main` C-ABI export reached through
  `dlopen`/`libloading` (the lowest-level entry point there is) **and** the
  process entry of the built executable (the composed pipeline: libc start-up →
  `main` → stdio flush → exit status).  Every row is checked through **both**.
* **M — residual stdin state**: how much of stdin the process leaves behind for
  whoever else holds the same open file description.  glibc reads a whole
  `st_blksize` block and then `lseek`s the descriptor back from `_IO_cleanup()`
  (`_IO_new_file_sync`), which is observable as `{ ./driver; cat; } < input`;
  seekable vs. unseekable stdin behave differently and the block boundary is a
  special case in its own right.
* **N — `argv`**: `int main()` declares no parameters, so extra command-line
  arguments must be invisible.

## The table

Every row is driven with **many randomized inputs** (fixed seed `20260818`) and
compared byte-for-byte on **stdout**, **stderr**, **exit code**, **terminating
signal**, **core-dump flag** and **residual stdin offset**, through **both**
entry points of column *L*.  Row count per randomized row is `DRIVER_DIFF_CASES`
(default 120).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| R1 | `driver_main` + exe | canonical `"<x> <y>"`, both operands random full-range `i32`, `y != 0`, single space | [x] |
| R2 | `driver_main` + exe | small operands, all four sign quadrants, `-20..=20`, `y != 0` (exercises the glibc `numer>=0 && rem<0` fix-up branch) | [x] |
| R3 | `driver_main` + exe | exact division (`x` a random multiple of `y`) — remainder 0 | [x] |
| R4 | `driver_main` + exe | `|x| < |y|` — quotient 0, remainder `x` | [x] |
| R5 | `driver_main` + exe | `x == 0` with random `y != 0`; and `y == ±1` with random `x` | [x] |
| R6 | `driver_main` + exe | `int` boundary operands: `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` × the same set, `y != 0`, excluding `INT_MIN ÷ -1` | [x] |
| R7 | `driver_main` + exe | whitespace matrix: random runs of ` \t\n\v\f\r` before / between / after the two numbers (axis A × E) | [x] |
| R8 | `driver_main` + exe | explicit `+`/`-` signs in all four combinations, incl. `-0` and `+0` as `x` (axis B × C) | [x] |
| R9 | `driver_main` + exe | 1…40 random leading zeros on either or both operands (axis D × G) | [x] |
| R10 | `driver_main` + exe | operands straddling the `int` range: uniformly sampled in `±2^33` so `long`→`int` truncation fires (axis D) | [x] |
| R11 | `driver_main` + exe | operands straddling the `long` range: sampled in `±(2^63 + 8)` so `strtol` clamping fires (axis D) | [x] |
| R12 | `driver_main` + exe | very long digit strings: 100 … 5000 random digits, optional sign (axis D) | [x] |
| R13 | `driver_main` + exe | one token only, then EOF / whitespace / non-digit — `y` keeps `1` (axis E × F) | [x] |
| R14 | `driver_main` + exe | more than two tokens: 3–6 numbers; only the first two are consumed (axis E) | [x] |
| R15 | `driver_main` + exe | glibc prefix shapes: `0`, `00`, `0x10`, `0X10`, `010`, `0b1`, `0e1` in either position (axis G) | [x] |
| R16 | `driver_main` + exe | numbers terminated by a non-digit rather than whitespace: `5.5 2`, `5e3 2`, `12,34`, `7q`, `1-2`, `3+4` (axis F) | [x] |
| R17 | `driver_main` + exe | numbers terminated by NUL or by a byte ≥ 0x80 (axis F) | [x] |
| R18 | `driver_main` + exe | `\r\n` (CRLF) and bare `\r` separators, and a trailing newline vs none (axis A) | [x] |
| R19 | `driver_main` + exe | **fuzz:** random byte strings, length 0…24, drawn from the full 0…255 alphabet (axes A–G jointly) | [x] |
| R20 | `driver_main` + exe | **fuzz:** random byte strings drawn from the hostile alphabet `0-9 + - space \t \n \v \f \r x X e E . , / \0 \xff a Z` | [x] |
| R21 | exe | stdin kind × content: pipe / regular file / `/dev/null` / closed fd 0 (axis I) | [x] |
| R22 | exe | stdout kind: pipe / regular file / `/dev/null` / `/dev/full` / closed fd 1 / reader-closed pipe (axis J) | [x] |
| R23 | exe | inherited `SIGPIPE` `SIG_DFL` vs `SIG_IGN`, crossed with the writable and the broken-pipe stdout of R22 (axis J × K) | [x] |
| R24 | exe | inherited `SIGFPE` `SIG_DFL` / `SIG_IGN` / blocked / ignored+blocked, crossed with `y == 0`, `INT_MIN ÷ -1` and a normal division (axis H × K) | [x] |
| R25 | `driver_main` + exe | return value of the entry point itself is `0` for every non-fatal row (`int main(){… return 0;}`) | [x] |
| R26 | `driver_main` | the `.so` is `dlopen`ed, `driver_main` resolved by name via `libloading`, called, and the library then closed — for both libraries, with identical harness code | [x] |
| R27 | exe | extra `argv`: none / 1 / 5 args / non-UTF-8-ish args, crossed with valid, invalid and trapping input (axis N) | [x] |
| R28 | `driver_main` + exe | residual **offset** on a seekable stdin: short inputs, multi-block inputs, and the `st_blksize` block boundary at ±1 byte (pads 4090/4093…4098, 8190…8193) so the `ungetc` lands on either side of the buffer edge; plus the trapping inputs, where the process dies before `_IO_cleanup` runs and therefore must **not** rewind (axis M) | [x] |
| R29 | `driver_main` + exe | residual **byte count** on an unseekable (pipe) stdin, where `_IO_new_file_sync`'s `lseek` fails with `ESPIPE` and glibc swallows it — so a whole block stays consumed (axis M) | [x] |

**In addition, axis M is measured on EVERY row**: the default stdin kind is a
regular file and `Outcome.stdin_residual` records the final file offset, so all
59 rows compare it alongside stdout/stderr/exit code/signal/core flag.

## Rows that are intentionally absent

* No "option/flag/mode" rows: the C exposes none (verified by grep — no `argc`,
  `argv`, `getenv`, `switch` or `if` anywhere in the translation unit).  R27
  nevertheless proves that passing arguments anyway changes nothing.
* No "byte order", "element type" or "count/stride" rows: the API passes no
  buffers, so there is no width, endianness or element-type axis.
* No multi-call / re-entrancy rows: `driver_main` consumes stdin to
  completion for its two conversions; the C program calls it once and so does
  the harness (one fresh child process per case, exactly like the executable).
