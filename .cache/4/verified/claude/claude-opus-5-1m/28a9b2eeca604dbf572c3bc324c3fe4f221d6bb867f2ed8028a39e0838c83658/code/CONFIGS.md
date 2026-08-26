# CONFIGS.md — Phase B configuration surface

Derived mechanically from `c_src/src/main.c`, `c_src/CMakeLists.txt` and
`Cargo.toml`.

## Axis 0 — build-time configuration (compile-time surface)

| source | switches found | conclusion |
|--------|----------------|------------|
| `c_src/src/main.c` | `grep -cE '#if|#ifdef|#ifndef|#else|#elif'` → **0** | no conditional compilation at all |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `target_compile_definitions`, single `add_executable(driver src/main.c)` | exactly one C build configuration |
| `Cargo.toml` `[features]` | `default = []` and no other feature | exactly one Rust feature combination |

**Complete enumeration of valid feature combinations** (Phase A / Phase D
requirement) — the cross-product of an empty feature set has one element:

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo check/test --no-default-features` | the empty (only) combination |
| 2 | `cargo check/test` (default) | identical to #1, since `default = []` |
| 3 | `cargo check/test --all-features` | identical to #1, since no features exist |

## Axis 1 — public entry points (the FULL set, from the C `.so` exports)

There is no header file; the public API is exactly what `nm -D` reports as
defined, i.e. the four non-`static` C functions:

| entry point | signature | level |
|-------------|-----------|-------|
| `printLine` | `void printLine(const char *)` | **lowest level** (the primitive every other function is built on) |
| `bad` | `void bad(void)` | composed of one `printLine` call |
| `good` | `void good(void)` | composed of `printLine` + `static helperGood()` |
| `main` | `int main(int, char **)` | top-level one-shot pipeline (6 `printLine`-level steps) |

## Axis 2 — runtime options / modes

`grep -nE 'if|switch|while|for|\?' c_src/src/main.c` yields a **single** branch:
`printLine`'s `if (line != NULL)`. There is no global state, no setter, no
option struct, no flag, no mode, no byte-order or format selector, and no
initialisation function. The only runtime axis is therefore the *shape of the
`const char *` argument* plus the *call sequence*.

## Axis 3 — input shapes the C code distinguishes

| shape | why the C distinguishes it |
|-------|----------------------------|
| NULL vs non-NULL | the `if (line != NULL)` branch |
| length 0 / 1 / small / large / crossing `BUFSIZ` (4096, 8192) | `puts` copies through the stdio buffer; buffer-boundary behaviour |
| byte values: ASCII printable / control / high (`0x80`–`0xFF`, invalid UTF-8) | raw `char*` copy, no encoding validation (Rust must not use `&str`) |
| `printf` directives inside the data (`%s`, `%n`, `%%`) | fixed `"%s\n"` format ⇒ data is never interpreted |
| embedded `\n`/`\r`/`\t` | output must still get exactly one appended `\n` |
| call sequence / repetition / interleaving | output ordering and absence of hidden state |

## Configuration-surface table

Each row is a meaningful combination of {entry point} × {options: none} ×
{input shape}. Every row is exercised through **both** `.so` files via
`libloading`, comparing captured fd-1 bytes exactly. Rows marked *randomized*
run many property-style inputs from a fixed seed (`SEED = 0x2026_0818`), not a
single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `printLine` | non-NULL, empty string `""` (length 0) | `cfg_c1_print_line_empty` | [x] |
| C2 | `printLine` | non-NULL, single ASCII byte (all 95 printable values) | `cfg_c2_print_line_single_ascii` | [x] |
| C3 | `printLine` | non-NULL, short ASCII words; the exact literals used by the C source (`"bad()"`, `"good()"`, `"helperGood()"`, `"helperBad()"`, `"Calling good()..."`, `"Finished good()"`, `"Calling bad()..."`, `"Finished bad()"`) | `cfg_c3_print_line_c_literals` | [x] |
| C4 | `printLine` | non-NULL, *randomized* printable-ASCII strings, lengths 0–256 (500 cases) | `cfg_c4_print_line_random_ascii` | [x] |
| C5 | `printLine` | non-NULL, *randomized* full-byte-range strings (`0x01`–`0xFF`, invalid UTF-8 included), lengths 0–256 (500 cases) | `cfg_c5_print_line_random_bytes` | [x] |
| C6 | `printLine` | non-NULL, length exactly at stdio buffer boundaries: 4095/4096/4097 and 8191/8192/8193 | `cfg_c6_print_line_buffer_boundaries` | [x] |
| C7 | `printLine` | non-NULL, very large payloads: 64 KiB and 1 MiB of *randomized* bytes | `cfg_c7_print_line_large` | [x] |
| C8 | `printLine` | non-NULL, embedded newlines/CR/TAB/ESC/DEL and trailing+leading whitespace | `cfg_c8_print_line_control_bytes` | [x] |
| C9 | `printLine` | non-NULL, strings that look like format strings (`%s`, `%d`, `%n`, `%p`, `%%`, `%1000000d`) | `cfg_c9_print_line_format_like` | [x] |
| C10 | `printLine` | NULL (the guarded branch), on its own | `cfg_c10_print_line_null` | [x] |
| C11 | `bad` | no arguments; single call | `cfg_c11_bad_single` | [x] |
| C12 | `good` | no arguments; single call (must also emit the `static helperGood()` line) | `cfg_c12_good_single` | [x] |
| C13 | `main` | `argc = 1`, `argv = {"driver", NULL}` (normal one-shot pipeline) | `cfg_c13_main_normal` | [x] |
| C14 | `main` | `argc = 5`, `argv` with extra arguments (must be ignored) | `cfg_c14_main_with_args` | [x] |
| C15 | `main` | repeated invocation (3×) — no state may accumulate; return value `0` each time | `cfg_c15_main_repeated` | [x] |
| C16 | `printLine`+`bad`+`good`+`main` | *randomized* interleaved call sequences (200 sequences of 1–20 calls, random shapes incl. NULL) — the composed pipeline, driven at the lowest level | `cfg_c16_random_call_sequences` | [x] |
| C17 | `bad`+`good` | repeated/alternating calls (50 iterations) — ordering and idempotence | `cfg_c17_bad_good_alternating` | [x] |
| C18 | whole program | end-to-end executable comparison: `c_src/build/driver` vs `target/debug/driver`, stdout+stderr+exit status, redirected to a **file** and through a **pipe** (different stdio buffering modes) | `cfg_c18_executables_end_to_end` | [x] |
| C19 | `printLine` | *randomized* strings whose bytes are drawn to include NUL-adjacent edge values (`0x01`, `0x7F`, `0x80`, `0xFF`) with high probability (300 cases) | `cfg_c19_print_line_edge_byte_mix` | [x] |
| C20 | `main` | `argv` array whose elements are non-ASCII/huge strings (must remain unread) | `cfg_c20_main_hostile_argv` | [x] |

All 20 rows pass under the single valid feature combination (verified for
`--no-default-features`, default, and `--all-features`, which are equivalent by
construction).
