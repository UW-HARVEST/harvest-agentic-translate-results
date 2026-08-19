# CONFIGS.md — Phase B configuration-surface table

## Build-time configuration surface

| axis | values | evidence |
|------|--------|----------|
| Cargo `[features]` | **none declared** → exactly **1** valid combination (the default/empty one) | `Cargo.toml`; `cargo check --no-default-features --all-targets` passes |
| C preprocessor conditionals | **none** | `grep -nE '#\s*(if\|ifdef\|ifndef\|else\|elif\|endif)' c_src/src/main.c` → no matches |
| CMake options | **none** | `grep -nE 'option\|if\(\|add_definitions\|target_compile' c_src/CMakeLists.txt` → no matches; `CMakeLists.txt` is just `add_executable(driver src/main.c)` |

So Phases B and C need to be run for exactly **one** feature combination, and
"repeat for every feature combination" is satisfied by that single run.
`scripts/verify.sh` still drives it through the `--no-default-features` form so the
enumeration is explicit and machine-checked rather than assumed.

## Runtime configuration surface

There is likewise **no** runtime option surface: `main` ignores `argc`/`argv` (they are
stored to the stack and never read), the program reads no environment variable, opens no
file, and has no mode flag or global state. The only inputs are:

1. **which entry point** is invoked (5 exported symbols), and
2. **the bytes on stdin**.

There is one more axis that is easy to miss and that turned out to matter a great deal:
**what kind of file descriptor stdin and stdout are attached to**. C stdio branches on it
(`stdout` is line-buffered on a terminal and block-buffered otherwise), and because
`bad()` can kill the process mid-run, that buffering mode is directly observable in how
much output survives. It also decides whether a seekable `stdin` gets repositioned at
exit. Rows 40-48 cover it.

The axes below are therefore the entry points crossed with the input shapes the C code
actually distinguishes, crossed with the descriptor kinds.

### Axis A — entry point (all 5 exported symbols, lowest-level first)

| entry point | reads stdin? | lines consumed | internal calls |
|-------------|--------------|----------------|----------------|
| `printIntLine(int)` | no | 0 | leaf |
| `printLine(const char*)` | no | 0 | leaf |
| `bad()` | yes | 1 | `printLine`, `printIntLine` |
| `good()` | yes | 1 (by `goodB2G`; `goodG2B` reads none) | `goodG2B`, `goodB2G` |
| `main(int, char**)` | yes | 2 (`goodB2G` then `bad`) | `good`, `bad`, `printLine` |

`goodG2B` and `goodB2G` are `static` and only reachable through `good()`; they are
exercised transitively (rows 20–23).

### Axis B — `data` value class (what the index-dispatch actually branches on)

Derived from the frame layout in `src/imp.rs` (read out of `objdump -d`), which is what
makes these classes *distinct* rather than arbitrary:

| class | range | behavior in `bad()` (unchecked sink) | behavior in `goodB2G()` (checked sink) |
|-------|-------|--------------------------------------|----------------------------------------|
| neg   | `< 0`        | "negative" message | "out-of-bounds" message |
| in    | `0..=9`      | `buffer[data]=1`, ten values printed | same |
| pad   | `10`         | store into alignment padding — benign | "out-of-bounds" message |
| ibuf  | `11..=13`    | store into dead `inputBuffer` — benign | "out-of-bounds" message |
| i     | `14`         | store into `i`, immediately overwritten by `i=0` — benign | "out-of-bounds" message |
| dataslot | `15`      | store into `data`, already latched in `%eax` — benign | "out-of-bounds" message |
| rbp   | `16..=17`    | overwrites `bad`'s saved rbp → fatal **iff the caller reloads `rbp`** (it does in the executable); the fault lands as `main` returns, so all output is already out | "out-of-bounds" message |
| ret   | `18..=19`    | overwrites `bad`'s **own** return address → fatal for **every** caller, at `bad`'s `ret`, so `"Finished bad()"` is never printed | "out-of-bounds" message |
| caller | `20..=25`   | store into `main`'s argc/argv/saved-rbp — benign in the executable | "out-of-bounds" message |
| callerret | `26..=27` | overwrites `main`'s return address → fatal in the executable, after all output | "out-of-bounds" message |
| far   | `28 ..` probed limit | store above `main`'s frame, still inside the mapping — benign | "out-of-bounds" message |
| band  | probed limit `..` ~2500 | **nondeterministic in C** (stack ASLR) — not asserted | "out-of-bounds" message |
| offstack | `≥ 50_000`   | store past the end of the stack mapping → fatal; `SIGSEGV` reproducibly below `10^6`, `SIGSEGV`-or-`SIGBUS` above | "out-of-bounds" message |

### Axis C — descriptor kind (what C stdio branches on)

| axis | values | what it changes in C |
|------|--------|----------------------|
| stdout kind | terminal / pipe / regular file / **pipe with the reader closed** | line- vs block-buffering, hence how much output survives a crash; a closed reader raises `SIGPIPE` (default disposition ⇒ death by signal 13) |
| stdin kind | pipe / **seekable regular file** / `/dev/null` / closed fd | glibc repositions a *seekable* stdin to the logical read offset at exit, so a later reader of the same descriptor sees the unread remainder |
| caller of the `.so` | none (executable) / a C consumer that also uses `printf` and `fgets` | the consumer shares the same `FILE` buffers, so output interleaves in call order and reads cooperate |

### Axis D — stdin byte-shape

`fgets(buf, 14, stdin)` ⇒ at most 13 bytes per call. Shapes the code distinguishes:
empty/EOF · `"\n"` only · whitespace-only · ≤13 bytes with newline · **exactly 13 bytes**
(fills the buffer, newline not reached) · **>13 bytes** (truncated; remainder feeds the
*next* `fgets`) · no trailing newline at EOF · embedded NUL · leading whitespace
(` \t\n\v\f\r`) · leading `+`/`-` · leading zeros · non-numeric prefix/suffix ·
CRLF · values straddling `INT_MAX`/`INT_MIN`/`LONG_MAX`.

## The table

One row per combination the C treats differently. Each row is driven with **many
randomized inputs** from a fixed seed (`tests/common/mod.rs`, seed `0x5EED_129`), not a
single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `printIntLine` | randomized `int` over full `i32` range (incl. `0`, `±1`, `INT_MAX`, `INT_MIN`) | [x] |
| 2  | `printIntLine` | called repeatedly in a loop — ordering/buffering of successive lines | [x] |
| 3  | `printLine` | randomized ASCII strings, length 0..64 | [x] |
| 4  | `printLine` | strings containing `%d`/`%s`/`%n` (format-specifier bytes must pass through verbatim) | [x] |
| 5  | `printLine` | randomized non-UTF-8 byte strings (high bytes `0x80..0xff`) | [x] |
| 6  | `printLine` | strings containing embedded `\n` and `\t` | [x] |
| 7  | `printLine`, `printIntLine` | interleaved calls — relative output order | [x] |
| 8  | `bad` (via `.so`) | stdin = single line, `data` class **in** (`0..=9`), randomized | [x] |
| 9  | `bad` (via `.so`) | stdin = single line, `data` class **neg**, randomized negative | [x] |
| 10 | `bad` (via `.so`) | stdin = single line, `data` classes **pad/ibuf/i/dataslot** (`10..=15`) | [x] |
| 11 | `bad` (via `.so`) | stdin = EOF (empty) → `fgets` NULL path | [x] |
| 12 | `bad` (executable) | `data` = every value from `-8` up to the probed deterministic limit (≤200), exhaustive, 8 runs each to prove reproducibility | [x] |
| 13 | `bad` (executable) | `data` classes **rbp/ret** (`16..=19`) — SIGSEGV + empty stdout (on a pipe); byte counts differ per class on a tty, see row 40 | [x] |
| 14 | `bad` (executable) | `data` classes **caller** (`20..=25`) — benign | [x] |
| 15 | `bad` (executable) | `data` class **callerret** (`26..=27`) — SIGSEGV + empty stdout | [x] |
| 16 | `bad` (executable) | `data` class **far** (`28 ..` probed limit), randomized — benign | [x] |
| 17 | `bad` (executable) | `data` class **offstack**: `50_000..10^6` asserts exact `SIGSEGV` + empty stdout; `>10^6` incl. `INT_MAX` asserts death + empty stdout only | [x] |
| 18 | `good` (via `.so`) | stdin single line, `data` class **in** — both `goodG2B` and `goodB2G` print | [x] |
| 19 | `good` (via `.so`) | stdin single line, `data` classes **neg** / `≥10` — `goodG2B` prints, `goodB2G` rejects | [x] |
| 20 | `good` (via `.so`) | stdin = EOF → `goodG2B` still prints, `goodB2G` takes the NULL path | [x] |
| 21 | `main` (via `.so`) | `argc=1`, valid `argv`; stdin = 2 well-formed lines, randomized in `0..=9` | [x] |
| 22 | `main` (via `.so`) | `argc=0`, `argv=NULL`; identical stdin — args are ignored | [x] |
| 23 | `main` (executable) | stdin = 2 lines, randomized cross-product of `data` classes for line 1 × line 2 | [x] |
| 24 | executable | stdin = **0 lines** (immediate EOF): both `fgets` calls fail | [x] |
| 25 | executable | stdin = **1 line** only: `goodB2G` consumes it, `bad`'s `fgets` hits EOF | [x] |
| 26 | executable | stdin = **>2 lines**: extra lines never read | [x] |
| 27 | executable | line of **exactly 13 bytes** (buffer exactly full, no newline consumed) | [x] |
| 28 | executable | line of **>13 bytes** — truncation, remainder feeds `bad`'s `fgets` | [x] |
| 29 | executable | single long digit run (e.g. 26 digits) spanning both `fgets` calls | [x] |
| 30 | executable | leading-whitespace forms ` `, `\t`, `\v`, `\f`, `\r` before digits | [x] |
| 31 | executable | explicit `+` / `-` sign, and `-0` | [x] |
| 32 | executable | leading zeros (`"0000000000007"`) | [x] |
| 33 | executable | non-numeric and mixed (`"abc"`, `"3abc"`, `"abc3"`, `"0x10"`, `"1e3"`, `"1.9"`, `"--5"`, `"++5"`) | [x] |
| 34 | executable | CRLF line endings (`"7\r\n"`) | [x] |
| 35 | executable | no trailing newline on the last line | [x] |
| 36 | executable | embedded NUL bytes in a line | [x] |
| 37 | executable | values straddling `INT_MAX` / `INT_MIN` / 13-digit `long` range | [x] |
| 38 | executable | fully randomized byte soup from the alphabet `0-9 + - space tab nl . e x X NUL a b z`, 0..30 bytes | [x] |
| 39 | executable | fully randomized `i32` pairs across all `data` classes | [x] |

| 40 | executable | stdout on a **pseudo-terminal**, `bad()` index in each class — line-buffered, so the surviving byte count differs per class (167 / 151 / 121) | [x] |
| 41 | executable | stdout on a **pipe**, same indices — block-buffered, so a crash yields 0 bytes | [x] |
| 42 | executable | stdout is a pipe whose **read end is closed** — `SIGPIPE`, status 13 | [x] |
| 43 | executable | stdin is a **seekable regular file** shared with a later reader — descriptor left at the logical offset | [x] |
| 44 | executable | stdin is `/dev/null` (immediate EOF) and a **closed** fd 0 | [x] |
| 45 | C consumer + `.so` | consumer's `printf` interleaved with `printLine` / `printIntLine` — shared `FILE` buffer, call order preserved | [x] |
| 46 | C consumer + `.so` | consumer's own `fgets` followed by `good()` and `bad()` — shared stream position | [x] |
| 47 | C consumer + `.so` | exported `bad()` across every index class 0..200 | [x] |
| 48 | C consumer + `.so` | consumer buffers output, then `bad()` dies — buffer lost, as C's is | [x] |

## Known-unmatchable regions (documented, not swept under the rug)

Three things about the out-of-bounds write cannot be matched, and in each case the reason
is that **C itself is not deterministic**, so no implementation could match them.

### 1. Whether a far write faults (stack ASLR)

The benign/fatal boundary is **not reproducible in C itself**. Measured over 40 runs per index
with the CMake-built binary, index `1500` crashed 25/40 times and index `2200` 37/40:
stack ASLR moves the top of the `[stack]` mapping relative to `bad`'s frame, so whether
`buffer[k] = 1` faults is random for `k` in roughly `1400..2500` (the exact band shifts
with the size of the environment block). No implementation can be byte-identical there.

`src/imp.rs` therefore derives the boundary from the live `[stack]` mapping in
`/proc/self/maps`, which tracks the environment automatically. Measured crash rates over
30 runs per index, C vs Rust, in the same environment:

| k | 1000 | 2500 | 3000 | 3400 | 3600 | 4000 | 4400 | 4800 | 5200 | 6000 | 50000 |
|---|------|------|------|------|------|------|------|------|------|------|-------|
| C    | 0% | 0% | 0% | 13% | 23% | 43% | 66% | 86% | 96% | 100% | 100% |
| Rust | 0% | 0% | 0% | 3%  | 26% | 43% | 43% | 83% | 96% | 100% | 100% |

Both are 0% below the band and 100% above it, and agree inside it to within the sampling
noise of 30 trials. Tests assert equality **outside** the band and deliberately do not
assert inside it. The safe boundary is **probed at run time** by
`tests/common::deterministic_benign_limit`, because it moves with the size of the
environment block: with an empty environment even index 500 faults on ~2 runs in 12,
while under a typical inherited environment index 1300 is still benign 12/12.
`tests/oob_band.rs` asserts the *statistical* agreement above, so a regression that
removed the emulation entirely would still be caught rather than silently passing.

### 2. *Which* fatal signal a far write raises

Once `4 * k` reaches the tens of megabytes, whether the fault surfaces as `SIGSEGV` or
`SIGBUS` also depends on the ASLR layout. Measured on the C binary with 120 random
samples per decade: `50_000..1_000_000` and `1_000_000..10_000_000` were `SIGSEGV`
120/120 each; `10_000_000..100_000_000` produced one `SIGBUS`; at `i32::MAX` C is 33%
`SIGBUS` / 67% `SIGSEGV` over 30 runs. Tests therefore assert the exact signal only below
`SIGNAL_KIND_UNSTABLE_MIN = 1_000_000`, and above it assert only "died from a fatal signal
with no output" — pinning the number there would be asserting against C's own coin flip.

### 3. Indices that reach a *caller's* frame through the `.so` export

Index 16/17 clobbers `bad()`'s saved `rbp` and 26/27 the return address of the frame
above. Whether that is fatal depends on the **caller's code generation**: a `gcc -O0`
caller keeps its frame pointer in `rbp` and dies, an optimized caller that never reloads
`rbp` survives untouched. Both were observed — the same C `.so` at index 16 dies when
called from the `gcc -O0` consumer and exits cleanly when called from the optimized Rust
test harness.

Only index 18/19, which clobbers `bad()`'s own **return address**, is fatal for every
caller. So `imp.rs` splits the two cases with a `Caller` enum: the executable and the
exported `main` know their caller is this program's `main` (fatal set `{16,17,18,19,26,27}`,
verified exhaustively), while the exported `bad()` treats only `{18,19}` as fatal rather
than fabricating a fault its caller need not suffer. `tests/stdio_semantics.rs` checks
both callers.
