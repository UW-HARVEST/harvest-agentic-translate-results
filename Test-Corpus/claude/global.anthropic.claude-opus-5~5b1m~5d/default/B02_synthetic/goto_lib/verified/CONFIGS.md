# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from every branch the C code takes on valid input.

## Axes the C actually branches on

**Runtime options / modes:** none. The library exposes no flags, no global
state, no `#ifdef`-selected behaviour and no init/config call. The *only*
"configuration" is the value of the two arguments, so the axes below are the
input shapes.

**Public entry points (all three, lowest-level first):**

| level | symbol | signature | source |
|-------|--------|-----------|--------|
| low | `forward_goto_example` | `int (int)` | `goto.c:29` |
| low | `open_with_cleanup` | `FILE* (const char*)` | `goto.c:42` |
| high (wrapper) | `driver` | `int (int, const char*)` | `goto.c:65`, only symbol in `goto.h` |

`driver` is the convenience wrapper; the two low-level functions are tested
directly as well, per Phase B.

**Axis 1 — `int x` / `int num` shape** (branch: `if (x < 0)`, arithmetic `x * 2`):
`0` · small positive · large positive · `INT_MAX/2` (largest non-overflowing)
· `> INT_MAX/2` (signed overflow in `x * 2`) · `INT_MAX` · negative · `INT_MIN`.

**Axis 2 — `const char* filename` shape** (branches: `if (!fp)`, the
`while (fgets(...))` loop trip count, `if (ferror(fp))`):
missing path · empty file (0 loop iterations) · single short line · single line
with **no trailing newline** · many lines · a line exactly `98`/`99`/`100`/`101`
bytes long (the `sizeof(buffer) == 100` chunking boundary, where one logical
line is split across multiple `fgets` calls) · content containing embedded NUL
bytes (`printf("%s")` truncates at the NUL while `fgets` consumed more) ·
content containing `%` characters (they reach `printf` only as the *argument*
of `"%s"`, so they must **not** be interpreted) · large file (many buffer
refills) · a directory (`fopen` succeeds, `fgets` sets `ferror`) · NULL /
unreadable (error rows, see `ERRORS.md`).

**Axis 3 — resource/return-value shape:** whether the returned `FILE*` is
non-NULL and must be `fclose`d by the caller (`open_with_cleanup` success) vs.
NULL (`driver` closes it internally on success). Exercised by asserting
NULL-ness parity and by closing the handle in the test.

## Table (cross-product, pruned to combinations the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| A1 | `forward_goto_example` | `x == 0` — boundary, not `< 0`; prints `Processing: 0`, returns `0` | [x] |
| A2 | `forward_goto_example` | `x` small positive (1, 2, 7, …) randomized | [x] |
| A3 | `forward_goto_example` | `x` large positive randomized in `1..=INT_MAX/2` (no overflow in `x*2`) | [x] |
| A4 | `forward_goto_example` | `x == INT_MAX/2 == 1073741823` — largest `x` with non-overflowing `x*2` | [x] |
| A5 | `forward_goto_example` | `x > INT_MAX/2` incl. `INT_MAX` — signed overflow in `x * 2`; must match the C codegen's two's-complement wrap | [x] |
| A6 | `forward_goto_example` | full-range randomized `i32` sweep (mixes the `x < 0` branch with all of the above) | [x] |
| B1 | `open_with_cleanup` | existing file, **empty** (0 bytes) → loop body never runs, `ferror == 0`, returns non-NULL handle | [x] |
| B2 | `open_with_cleanup` | existing file, single short line **with** trailing `\n` | [x] |
| B3 | `open_with_cleanup` | existing file, single short line **without** trailing `\n` (last `fgets` returns a partial line) | [x] |
| B4 | `open_with_cleanup` | existing file, many short lines (randomized count 2..50, randomized lengths) | [x] |
| B5 | `open_with_cleanup` | line length exactly `98` bytes + `\n` → fits in one `fgets` chunk (99 chars + NUL) | [x] |
| B6 | `open_with_cleanup` | line length exactly `99` bytes + `\n` → `\n` lands in the *next* `fgets` chunk (buffer-boundary split) | [x] |
| B7 | `open_with_cleanup` | line length exactly `100`/`101`/`199`/`200` bytes → 2–3 `fgets` chunks per logical line | [x] |
| B8 | `open_with_cleanup` | content with **embedded NUL bytes** → `printf("%s", buffer)` stops early; output is *not* the file content | [x] |
| B9 | `open_with_cleanup` | content full of `%` / `%s` / `%n` conversion specifiers → passed as the `%s` argument, must not be interpreted as a format | [x] |
| B10 | `open_with_cleanup` | large file (≥ 64 KiB, many buffer refills), randomized bytes with newlines | [x] |
| B11 | `open_with_cleanup` | file of only newlines (`\n\n\n…`) → many 1-byte lines | [x] |
| B12 | `open_with_cleanup` | file whose content has no newline at all and is longer than 99 bytes → pure chunking, no line structure | [x] |
| B13 | `open_with_cleanup` | randomized binary content (arbitrary bytes incl. NUL, `\r`, high-bit bytes), randomized sizes 0..4096 | [x] |
| B14 | `open_with_cleanup` | success path, returned `FILE*` handle: assert both are non-NULL and both are `fclose`-able by the caller | [x] |
| C1 | `driver` | `num == 0` + empty file → returns `0`, stdout `Processing: 0\nGoto output: 0\n` | [x] |
| C2 | `driver` | `num > 0` + multi-line file → returns `0`, stdout `Processing:`/`Goto output:` then the file content | [x] |
| C3 | `driver` | `num > INT_MAX/2` (overflowing `num*2`) + valid file → the printed `Goto output` is the wrapped value | [x] |
| C4 | `driver` | `num == INT_MAX` + valid file | [x] |
| C5 | `driver` | randomized `num` (full `i32` range) × randomized file shape from axis 2 — the composed pipeline, ordering of stdout writes from `forward_goto_example` vs. `open_with_cleanup` included | [x] |
| C6 | `driver` | `num >= 0` + buffer-boundary file (B6/B7 shapes) — interaction of the two axes | [x] |
| C7 | `driver` | `num >= 0` + file with embedded NULs / `%` specifiers (B8/B9 shapes) | [x] |
| C8 | `driver` | `num >= 0` + directory as `filename` → `Processing:`+`Goto output:` on stdout, then `-2` | [x] |
| C9 | `driver` | interleaving / stream-buffering order: stdout and stderr both written in one call (`num < 0` writes only stderr; `num >= 0` + bad file writes both, stdout first) | [x] |
| C10 | `driver` | called repeatedly in sequence on the same file (no hidden per-call state; output must be the concatenation) | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` section, so the set of feature combinations is
`{ default }` == `{ --no-default-features }`. `check_feature_combos.sh`
enumerates and runs them all.

## Binary executable

`CMakeLists.txt` builds only `add_library(driver SHARED src/goto.c)` — there is
no `add_executable`, and `Cargo.toml` declares only `[lib] crate-type =
["cdylib"]` with no `src/main.rs`. There is therefore no driver binary whose
stdout could be compared; stdout/stderr are captured and compared around the
FFI calls instead (see `tests/common/mod.rs`).
