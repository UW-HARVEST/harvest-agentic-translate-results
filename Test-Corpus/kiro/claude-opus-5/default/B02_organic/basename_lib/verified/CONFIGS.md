# CONFIGS.md — Configuration-surface table

Derived mechanically from `c_src/src/lib.c`. The full body is:

```c
char *tool_basename(char *path)
{
  char *s1;
  char *s2;

  s1 = strrchr(path, '/');
  s2 = strrchr(path, '\\');

  if(s1 && s2) {
    path = (s1 > s2) ? s1 + 1 : s2 + 1;
  }
  else if(s1)
    path = s1 + 1;
  else if(s2)
    path = s2 + 1;

  return path;
}
```

## Axes the C actually branches on

**Runtime options / modes / flags:** NONE. The public header is one line
(`char *tool_basename(char *path);`) — there is no context struct, no setopt
call, no flags word, no enum, and no `#ifdef` in either file. So the
configuration cross-product collapses onto the input-shape axes alone.

**Public entry points:** exactly one — `tool_basename`. There is no
higher-level convenience wrapper and no lower-level helper exported (glibc
`strrchr` is an import, replicated as a private Rust `fn`), so "exercise the
lowest-level entry point" and "exercise the top-level entry point" are the same
call here. Every row below calls `tool_basename` through the `.so` export.

**Control-flow axes (from the source, not assumed):**

| axis | values the code distinguishes |
|------|-------------------------------|
| A. `s1` = last `'/'` | NULL (no `/`) vs non-NULL |
| B. `s2` = last `'\\'` | NULL (no `\`) vs non-NULL |
| C. `s1 > s2` (pointer compare, only when both non-NULL) | true (`/` is later) vs false (`\` is later). `s1 == s2` is impossible since the bytes differ, so "false" means `s1 < s2`. |
| D. position of the winning separator | interior · index 0 (first byte) · last byte (result is the empty string) |
| E. separator multiplicity | one occurrence vs many (exercises "last", not "first") |
| F. string length | 0 · 1 · small · page-crossing / multi-MB |
| G. byte values in the non-separator part | ASCII · high bytes `0x80..=0xFF` (signed `c_char` hazard) · bytes adjacent to the separators (`0x2E/0x30/0x5B/0x5D`) · full random `0x01..=0xFF` |

## Configuration table

One row per combination the C treats differently. Every row is driven with
**many randomized inputs** from a fixed-seed PRNG (seed `0x5EED_1234_ABCD_0001`,
SplitMix64), not a single hand-picked value, and both `.so`s are called through
`libloading`. The asserted output is byte-for-byte: the returned pointer's offset
from the input buffer AND the full returned NUL-terminated byte string.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tool_basename` | no options exist; **empty string** `""` (len 0) → A=NULL, B=NULL, all `if`s false, returns `path` unchanged | [x] |
| 2 | `tool_basename` | **no separator at all**, ASCII, random len 1..64, random content → A=NULL, B=NULL, returns `path` unchanged | [x] |
| 3 | `tool_basename` | **no separator**, random bytes drawn from full `0x01..=0xFF` incl. high bytes ≥ `0x80` → signed-`c_char` comparison hazard | [x] |
| 4 | `tool_basename` | **only `'/'`, exactly one occurrence, interior** → A≠NULL, B=NULL, `else if(s1)` branch, returns `s1+1` | [x] |
| 5 | `tool_basename` | **only `'/'`, many occurrences (2..8), interior** → must select the *last*, not the first | [x] |
| 6 | `tool_basename` | **only `'/'`, at index 0** → returns `path+1` | [x] |
| 7 | `tool_basename` | **only `'/'`, as the final byte** (trailing separator) → returns pointer to the NUL, i.e. the empty basename `""` | [x] |
| 8 | `tool_basename` | **only `'\\'`, exactly one occurrence, interior** → A=NULL, B≠NULL, `else if(s2)` branch, returns `s2+1` | [x] |
| 9 | `tool_basename` | **only `'\\'`, many occurrences (2..8)** → must select the last | [x] |
| 10 | `tool_basename` | **only `'\\'`, at index 0** and, separately, **as the final byte** → `path+1` / empty basename | [x] |
| 11 | `tool_basename` | **both separators present, last `'/'` strictly after last `'\\'`** (`s1 > s2` true) → returns `s1+1` | [x] |
| 12 | `tool_basename` | **both separators present, last `'\\'` strictly after last `'/'`** (`s1 > s2` false) → returns `s2+1` | [x] |
| 13 | `tool_basename` | both/either separator present **plus decoy bytes one step off the separators** (`0x2E '.'`, `0x30 '0'`, `0x5B '['`, `0x5D ']'`) densely interleaved → off-by-one in the byte comparison would diverge here | [x] |
| 14 | `tool_basename` | **adjacent separator pairs** `"/\\"` and `"\\/"` and runs of mixed separators (`"//\\\\//"`), including a string composed *entirely* of separators → exercises axis C at distance 1 | [x] |
| 15 | `tool_basename` | **fully random bytes `0x01..=0xFF`, random length 0..256**, separators appearing only by chance — unbiased property test over all of axes A–E and G simultaneously (10 000 cases) | [x] |
| 16 | `tool_basename` | **oversized input**: page-crossing and multi-MB strings (len 4095, 4096, 4097, 65 536, 1 048 576) with separators placed at the first byte, the last byte, and random interior offsets → long-scan / pointer-arithmetic overflow | [x] |
| 17 | `tool_basename` | **length-1 strings**, exhaustively over every byte value `0x01..=0xFF` (covers `"/"`, `"\\"`, and all 253 non-separator singletons) → degenerate-length dispatch | [x] |
| 18 | `tool_basename` | **buffer aliasing / in-place contract**: the returned pointer must point *into the caller's own buffer* (same allocation, offset in `0..=len`) and the input buffer must be left unmodified — asserted for every case in every row above | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]`. The only combinations are the
default (empty) feature set and `--no-default-features`, which are identical.
`run_all_features.sh` enumerates the feature list mechanically from `Cargo.toml`
(so it will expand automatically if features are ever added) and runs the whole
suite across `{dev, release} × {default, --no-default-features}` — 4
configurations — checking symbol parity for each profile's `.so` as well. Every
row above is verified under each.

## Suite sensitivity (mutation check)

Passing tests only mean something if they can fail. Six deliberate defects were
injected into `src/lib.rs` one at a time and `run_all_features.sh` was re-run;
each was caught in every configuration, and the unmutated baseline passes:

| mutation | detected |
|----------|----------|
| `s1 > s2` → `s1 < s2` (wrong separator wins) | yes |
| `strrchr` → first match instead of last | yes |
| drop `+1` in the `else if s1` branch | yes |
| drop `+1` in the `else if s2` branch | yes |
| add a NULL check the C does not have | yes (Phase C only) |
| skip the scan's NUL terminator condition | yes |

