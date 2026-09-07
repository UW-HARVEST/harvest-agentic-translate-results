# CONFIGS.md — Configuration-surface table (valid inputs)

Mechanically derived from the branches the C actually takes.

## Axes the C code branches on

`c_src/src/lib.c` has exactly these decision points:

| source line | branch |
|---|---|
| `if(s1 && s2)` | both separators present |
| `(s1 > s2) ? s1 + 1 : s2 + 1` | which separator is later (pointer compare) |
| `else if(s1)` | only `/` present |
| `else if(s2)` | only `\` present |
| (fall-through) | neither present → return input pointer unchanged |

So the axes are:

- **A. presence of `/`** — absent / present
- **B. presence of `\`** — absent / present
- **C. relative order of the LAST `/` vs the LAST `\`** — only meaningful when
  both present: `/` later, `\` later (they can never be equal)
- **D. position of the winning separator** — first byte / interior / last byte
  (last byte ⇒ empty result component)
- **E. multiplicity** — one occurrence / several (only the last counts)
- **F. string length shape** — empty / 1 byte / small / large (64 KiB)
- **G. byte content** — ASCII / high-bit (≥ 0x80, signed-`char` sensitive) / all
  bytes 0x01–0xFF
- **H. trailing bytes after the NUL terminator** — absent / present and
  containing separators (must be ignored)

## Runtime options / modes / flags

**None.** `c_src/include/lib.h` declares a single function with a single
parameter. There is no init/config struct, no setter, no global, no
`#ifdef`-gated behaviour anywhere in `c_src`, and `translation/Cargo.toml`
declares **no `[features]` table** — so there is exactly one feature
combination (the default) and one code configuration to verify.

## Full set of public entry points

| entry point | level | in header | tested directly |
|---|---|---|---|
| `tool_basename` | lowest and only | yes | yes |

There is no convenience wrapper vs. low-level split; `tool_basename` *is* the
lowest-level entry point. The private helper `strrchr` is exercised
transitively through every row below (and is deliberately not exported by
either side, matching the C).

## Rows (pruned cross-product of the axes the C distinguishes)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) plus the hand-picked boundary values, comparing C and Rust
`.so` exports byte-for-byte on both the returned **string bytes** and the
returned **pointer offset** relative to the input buffer.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tool_basename` | no options; empty string `""` | [x] |
| 2 | `tool_basename` | no options; 1-byte non-separator, e.g. `"a"` | [x] |
| 3 | `tool_basename` | no options; 1-byte = `"/"` (separator is both first and last byte) | [x] |
| 4 | `tool_basename` | no options; 1-byte = `"\\"` | [x] |
| 5 | `tool_basename` | no separators, random ASCII, length 2..64 | [x] |
| 6 | `tool_basename` | only `/`, single occurrence, interior | [x] |
| 7 | `tool_basename` | only `/`, single occurrence, first byte | [x] |
| 8 | `tool_basename` | only `/`, single occurrence, last byte (empty component) | [x] |
| 9 | `tool_basename` | only `/`, many occurrences, random positions | [x] |
| 10 | `tool_basename` | only `/`, all bytes are `/` (e.g. `"////"`) | [x] |
| 11 | `tool_basename` | only `\`, single occurrence, interior | [x] |
| 12 | `tool_basename` | only `\`, single occurrence, first byte | [x] |
| 13 | `tool_basename` | only `\`, single occurrence, last byte (empty component) | [x] |
| 14 | `tool_basename` | only `\`, many occurrences, random positions | [x] |
| 15 | `tool_basename` | only `\`, all bytes are `\` | [x] |
| 16 | `tool_basename` | both present, last `/` strictly after last `\` (branch `s1 > s2`) | [x] |
| 17 | `tool_basename` | both present, last `\` strictly after last `/` (branch `s1 <= s2`) | [x] |
| 18 | `tool_basename` | both present, adjacent `"…/\\…"` (1-byte pointer delta, `\` wins) | [x] |
| 19 | `tool_basename` | both present, adjacent `"…\\/…"` (1-byte pointer delta, `/` wins) | [x] |
| 20 | `tool_basename` | both present, winning separator is the last byte (empty component) | [x] |
| 21 | `tool_basename` | both present, winning separator is the first byte | [x] |
| 22 | `tool_basename` | both present, many of each, random interleaving, length 2..64 | [x] |
| 23 | `tool_basename` | high-bit bytes (0x80..0xFF) mixed with separators — signed-`char` sensitivity | [x] |
| 24 | `tool_basename` | fully random bytes 0x01..0xFF (separators occur by chance), length 0..128 | [x] |
| 25 | `tool_basename` | buffer with trailing bytes *after* the NUL that contain separators (must be ignored) | [x] |
| 26 | `tool_basename` | large input, 64 KiB, separators only in the first half | [x] |
| 27 | `tool_basename` | large input, 64 KiB, separators only in the last 8 bytes | [x] |
| 28 | `tool_basename` | large input, 64 KiB of pure `/` | [x] |
| 29 | `tool_basename` | idempotence / composition: feed `tool_basename`'s own result back in (real-consumer pipeline, exercises the returned pointer as a valid input) | [x] |
| 30 | `tool_basename` | same buffer called repeatedly (no hidden state / no mutation of the input buffer — verified by comparing the buffer contents before and after) | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains only `add_library(driver SHARED src/lib.c)` —
no `add_executable`. `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` and has no `[[bin]]` / `src/main.rs`. **The project
builds no driver binary**, so the stdout-comparison item is not applicable.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section, therefore the complete
set of combinations is: `{default}` = `{}`. Verified with
`cargo test --release`, `cargo test --release --no-default-features`, and
`cargo test --release --all-features`, which are all the same configuration
here.
