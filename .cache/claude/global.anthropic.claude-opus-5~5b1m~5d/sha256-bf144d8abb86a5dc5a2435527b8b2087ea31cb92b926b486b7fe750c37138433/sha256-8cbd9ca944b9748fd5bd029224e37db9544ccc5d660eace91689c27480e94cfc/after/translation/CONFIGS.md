# CONFIGS.md — Phase A: configuration-surface table

## Axes actually present in the C source

Derived mechanically, not guessed:

* **Runtime options / modes / flags:** *none*. The only public entry point is
  `char *tool_basename(char *path)` (`c_src/include/lib.h` is one line). There
  is no context struct, no init/destroy pair, no setter, no global, no
  `#ifdef`, no `#define`, no enum:
  ```
  $ grep -nE '#if|#ifdef|#define|enum|extern|static' c_src/src/lib.c c_src/include/lib.h
  (no matches; exit status 1)
  ```
* **Public entry points (FULL set, lowest level included):** exactly one,
  `tool_basename`. It *is* the lowest-level entry point; there is no
  convenience wrapper layered on top of anything.
* **Branches the code takes** (`c_src/src/lib.c:13-19`) — this is the real
  configuration space, driven purely by input *shape*:
  | branch | condition |
  |--------|-----------|
  | B1 | `s1 && s2` and `s1 > s2` → `s1 + 1` |
  | B2 | `s1 && s2` and `s1 < s2` → `s2 + 1` |
  | B3 | `s1 && !s2` → `s1 + 1` |
  | B4 | `!s1 && s2` → `s2 + 1` |
  | B5 | `!s1 && !s2` → `path` unchanged |
* **Input shapes the code distinguishes:** presence of `'/'` (`s1`), presence of
  `'\\'` (`s2`), their relative order, count of each (`strrchr` = *last*
  occurrence, so ≥2 occurrences is a distinct shape from exactly 1), position
  (first byte / interior / last byte), total length (0, 1, many, oversized),
  byte values in the rest of the string (ASCII, high-bit / non-UTF-8, near-miss
  bytes `.`,`0`,`[`,`]`), and interior NUL (truncates the search).

Because there are no flags, the cross-product below is
`{branch} × {occurrence count} × {separator position} × {length class} × {byte
alphabet}`, pruned to the combinations the C actually treats differently.

## Configuration rows

Every row is exercised against **both** `.so`s with **many randomized inputs**
(fixed seed `0x5EED_1234`, deterministic xorshift PRNG) plus the listed
hand-built shape, and the returned pointer is compared as an **offset from the
shared input buffer** (both libraries are handed the *same* buffer, so equal
offsets ⇔ byte-identical pointers) together with the full remaining byte string.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `tool_basename` | B5: no separator at all; pure-ASCII, length 1..64, randomized | [x] |
| 2 | `tool_basename` | B5: no separator, length 0 (empty string) | [x] |
| 3 | `tool_basename` | B5: no separator, alphabet restricted to near-miss bytes `. 0 [ ] : ¦` | [x] |
| 4 | `tool_basename` | B3: exactly one `'/'`, no `'\\'`; separator interior, randomized position | [x] |
| 5 | `tool_basename` | B3: exactly one `'/'` at the **first** byte (`"/x..."`) | [x] |
| 6 | `tool_basename` | B3: exactly one `'/'` at the **last** byte (trailing separator → empty basename) | [x] |
| 7 | `tool_basename` | B3: **many** `'/'`, no `'\\'` (randomized count 2..16) — exercises `strrchr` picking the *last* | [x] |
| 8 | `tool_basename` | B3: string is exactly `"/"` (length 1, separator is whole string) | [x] |
| 9 | `tool_basename` | B4: exactly one `'\\'`, no `'/'`; interior, randomized position | [x] |
| 10 | `tool_basename` | B4: exactly one `'\\'` at first byte | [x] |
| 11 | `tool_basename` | B4: exactly one `'\\'` at last byte (trailing) | [x] |
| 12 | `tool_basename` | B4: **many** `'\\'`, no `'/'` (randomized count 2..16) | [x] |
| 13 | `tool_basename` | B4: string is exactly `"\\"` | [x] |
| 14 | `tool_basename` | B1: both present, last `'/'` **after** last `'\\'` (`s1 > s2`), randomized | [x] |
| 15 | `tool_basename` | B2: both present, last `'\\'` **after** last `'/'` (`s2 > s1`), randomized | [x] |
| 16 | `tool_basename` | B1/B2: both present and **adjacent** (`"a/\\b"` and `"a\\/b"`) — off-by-one in the pointer comparison | [x] |
| 17 | `tool_basename` | B1/B2: both present, **many of each**, interleaved randomly (count 1..12 each) — the general case | [x] |
| 18 | `tool_basename` | B1/B2: both separators, one of them at the final byte (trailing) while the other is interior | [x] |
| 19 | `tool_basename` | B1/B2: string made **only** of separators (randomized mix of `/` and `\`, length 1..16) | [x] |
| 20 | `tool_basename` | Any branch, **non-UTF-8 / full 1..255 byte alphabet** with randomized separators — bytes ≥ 0x80, no NUL | [x] |
| 21 | `tool_basename` | Any branch, **oversized** input: 64 KiB with no separator | [x] |
| 22 | `tool_basename` | Any branch, **oversized** input: 64 KiB with the only separator at the very last byte | [x] |
| 23 | `tool_basename` | Any branch, **oversized** input: 64 KiB with randomized separators throughout | [x] |
| 24 | `tool_basename` | Interior NUL in the buffer: search must stop at the first NUL and ignore later separators | [x] |
| 25 | `tool_basename` | Aliasing/idempotence: `tool_basename(tool_basename(p))` — result fed back in (basename of a basename), randomized | [x] |
| 26 | `tool_basename` | Repeat-call stability: same buffer called N times must give the same offset; buffer must be left unmodified | [x] |
| 27 | `tool_basename` | Fully random fuzz sweep: 20 000 randomized strings, length 0..80, alphabet `{a,b,/,\\,.,0,[,],0x80,0xFF}` — the pruned cross-product sampled exhaustively | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table** (verified: `grep -n
'^\[features\]' translation/Cargo.toml` → no match), so there is exactly one
build configuration. `--no-default-features` and the default build are the same
compilation. Both are nonetheless run by `run_all_features.sh`.

## Binary executable

The project builds **no** binary/driver executable: `c_src/CMakeLists.txt`
declares only `add_library(driver SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` (no
`[[bin]]`, no `src/main.rs`). The "compare stdout of the two binaries" gate is
therefore vacuous and is discharged by that absence, not skipped.
