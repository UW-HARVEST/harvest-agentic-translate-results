# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Mirror of `ERRORS.md`, for inputs the C **accepts**. Axes derived mechanically
from the branches the C actually takes.

## Axis enumeration

**A1 — entry points.** The full public API from `nm -D`, *including the
lowest-level one*. `printLine` is the low-level primitive; `bad`/`good` are the
mid-level wrappers; `driver` is the one-shot convenience wrapper. The test suite
drives all four directly.

| level | symbol | signature |
|-------|--------|-----------|
| low   | `printLine` | `void printLine(const char *line)` |
| mid   | `bad`       | `void bad(void)` |
| mid   | `good`      | `void good(void)` |
| top   | `driver`    | `void driver(int useGood)` |

**A2 — runtime options / modes / flags.** Grep of the public header and of every
`if`/`switch`/`#ifdef` in the C shows exactly **one** runtime option:
`driver`'s `int useGood`, a C-truthiness flag selecting the `good()` path
(non-zero) or the `bad()` path (zero). There is no global state, no
init/config/setter function, no environment variable, no `#ifdef`, no compile
option. The only other branch in the library is `printLine`'s null guard, whose
non-null side is the valid path (axis A3).

**A3 — input shapes the code distinguishes.** From `printLine`'s guard plus
`puts`/`%s` semantics:

* pointer validity: non-NULL (valid path; NULL is `ERRORS.md` #1)
* length: 0 / 1 / small / stdio-buffer boundaries (4095, 4096, 4097 around
  `BUFSIZ`-class buffering) / large (64 KiB, 1 MiB)
* byte content: ASCII / high bytes (`>= 0x80`, i.e. negative `char` on x86-64) /
  full `0x01..=0xFF` range / invalid UTF-8 / format-specifier bytes (`%s`, `%n`)
* interior `'\0'` (truncation point)
* embedded `'\n'`
* the two fixed strings the library owns: `"helperBad string"` (17 B, automatic,
  dangling) and `"helperGood1 string"` (19 B, `.data`, valid)

**A4 — call multiplicity / ordering.** empty (no call) / one / many; and
whether the `.data` buffer is stable across repeated `good()` calls and across
interleaving with `bad()` and `printLine`.

**A5 — feature combinations.** `translation/Cargo.toml` declares **no
`[features]` table**, so the only configuration is the default one. Enumerated
and confirmed by script (`scripts/all_features.sh`): the complete set is
`{default}`, plus `--no-default-features` which is identical (no default
features exist). Both are exercised.

**A6 — binary / driver executable.** `c_src/CMakeLists.txt` has `add_library(...
SHARED)` only — **no `add_executable`** — and `translation/Cargo.toml` has no
`[[bin]]` and no `src/main.rs`. The project builds **no executable**, so the
"compare C and Rust stdout of the binary" gate is vacuous. Stdout is instead
compared per-call by redirecting fd 1 around each `.so` invocation, which is a
strictly stronger check.

## Configuration-surface table

Pruned cross-product of A1 × A2 × A3 × A4 — one row per combination the C
treats differently. Every row is run with **many randomised inputs** (fixed seed
`0x5EED_C0DE`, deterministic SplitMix64) unless the row is inherently a single
fixed input, and asserts C and Rust stdout are **byte-identical**.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-NULL, length 0 (`""`) — minimal valid input | [x] |
| 2 | `printLine` | non-NULL, length 1, randomised over all 255 non-NUL byte values | [x] |
| 3 | `printLine` | non-NULL, random ASCII printable, random length 1..=64, 512 random cases | [x] |
| 4 | `printLine` | non-NULL, random bytes from full `0x01..=0xFF` (high/negative `char`, invalid UTF-8), random length 1..=256, 512 random cases | [x] |
| 5 | `printLine` | non-NULL, exhaustive single-byte sweep: every value `0x01..=0xFF` as a 1-byte string | [x] |
| 6 | `printLine` | non-NULL, stdio buffer boundary lengths 4095 / 4096 / 4097 and 8191 / 8192 / 8193, random content | [x] |
| 7 | `printLine` | non-NULL, large payloads 64 KiB and 1 MiB, random content | [x] |
| 8 | `printLine` | non-NULL, content containing `'\n'` at random positions (multi-line payload) | [x] |
| 9 | `printLine` | non-NULL, content containing format-specifier bytes `%s`, `%d`, `%n`, `%%`, `%99999d` at random positions | [x] |
| 10 | `printLine` | non-NULL, interior NUL: random prefix + `'\0'` + random suffix (truncation shape) | [x] |
| 11 | `printLine` | non-NULL, the library's own two literals passed explicitly: `"helperBad string"`, `"helperGood1 string"` | [x] |
| 12 | `printLine` | non-NULL, buffer whose bytes are all `0xFF` / all `0x01` / alternating, lengths 1..=32 | [x] |
| 13 | `good` | no options; single call — the well-defined `.data` path | [x] |
| 14 | `good` | no options; many calls (100×) — `.data` buffer stability / idempotence | [x] |
| 15 | `bad` | no options; single call — the CWE-562 dangling-pointer path | [x] |
| 16 | `bad` | no options; many calls (100×) — stability of the defective path | [x] |
| 17 | `driver` | `useGood` non-zero = `1` (canonical true) → `good()` composed path | [x] |
| 18 | `driver` | `useGood` = `0` (the only false value) → `bad()` composed path | [x] |
| 19 | `driver` | `useGood` non-zero, randomised over 1024 uniform random `i32` values (re-drawn to be non-zero) → `good()` | [x] |
| 20 | `driver` | `useGood` boundary values: `-1`, `2`, `-2`, `i32::MIN`, `i32::MIN+1`, `i32::MAX`, `i32::MAX-1`, `0x8000_0000u32 as i32`, `0x0001_0000`, `0xFFFF_0000u32 as i32` | [x] |
| 21 | `driver` | random *sequence* of 256 randomly chosen `useGood` values (mixing 0 and non-zero) in one capture — composed pipeline, checks no state leaks between the good and bad paths | [x] |
| 22 | mixed: `driver` + `good` + `bad` + `printLine` | random interleaving of all four entry points, 256 randomly chosen ops in one capture, random payloads for `printLine` — the full composed pipeline a real consumer produces | [x] |
| 23 | `printLine` after `good`/`bad` | ordering axis: `good()`, then `printLine(random)`, then `bad()`, then `good()` — asserts the `.data` string was not mutated by neighbouring calls | [x] |
| 24 | all four | "empty" multiplicity: load both `.so`s, resolve all 4 symbols, make **no** call — asserts zero bytes on stdout from mere `dlopen` (no constructor/static-init output) | [x] |

## Feature-combination matrix

| combo | `cargo check` | Phase B rows | Phase C rows |
|-------|---------------|--------------|--------------|
| default (no features declared) | pass | 24/24 | 15/15 |
| `--no-default-features` (identical: no features exist) | pass | 24/24 | 15/15 |
