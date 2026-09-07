# CONFIGS.md — Phase A configuration-surface table (valid inputs)

Mirror of `ERRORS.md`, for inputs the C **accepts**. Axes are derived from what
the C actually branches on, plus the state the library's observable behaviour
actually depends on.

## Axes, and where each comes from in the source

**A1 — entry point.** All four exported symbols are public. The call hierarchy
from `driver.c` is `driver` → {`good`, `bad`} → `printLine` → `puts`. Tests
drive the **lowest** level (`printLine`) directly as well as the mid level
(`bad`, `good`) and the top-level wrapper (`driver`) — bugs in the composed
pipeline are invisible to per-wrapper tests.

**A2 — `driver`'s `int useGood`** (`if (useGood)`, `driver.c:51`). Two code
paths, but the whole `int` domain is valid: `0` → `bad()`, any non-zero →
`good()`. Boundary values `1`, `-1`, `2`, `INT_MIN`, `INT_MAX` and randomized
non-zero values all belong to the non-zero class.

**A3 — `printLine`'s `const char *line` shape** (`if (line != NULL)`,
`driver.c:30`, then `puts`). Distinct shapes the code/`puts` treat differently:
empty, one byte, many bytes, first-byte-NUL with trailing garbage, interior NUL,
embedded `\n`, non-ASCII / all 256 byte values, and very long (oversized).

**A4 — dynamic-binding mode: `RTLD_LAZY` vs `RTLD_NOW`.** Not a source `#ifdef`,
but a real configuration of the `.so` that is **observable here**: the C `.so` is
linked with lazy PLT binding, so the *first* intra-library call through a
`R_X86_64_JUMP_SLOT` entry runs `_dl_runtime_resolve`, which overwrites several
hundred bytes of stack just below `driver`'s frame — including the exact word
`bad()` reads. Measured: `driver(0)` prints `\n` under `RTLD_LAZY` but a stale
stack address under `RTLD_NOW`. Both modes must match.

**A5 — call-site stack state / call sequence.** Because `bad()` reads
uninitialized stack (CWE-457), its output is a function of the residue at its
call site. Distinct states: a fresh call; a call after `good()` has stored its
`"string"` pointer into the same slot; a call after `printLine`; a repeated
call; and long mixed sequences over a dirtied stack.

**A6 — first vs subsequent invocation** (interacts with A4): the PLT resolver
runs only once per process, so `driver(0); driver(0)` produces two *different*
lines. Sequences therefore include repeats.

## Comparison methodology (required for these rows to be meaningful)

`bad()` prints raw stack residue, so a naive C-vs-Rust comparison measures the
*harness*, not the library. Two artifacts had to be eliminated (both diagnosed
during Phase A, neither a translation defect):

1. **argv/env length.** The stale pointer aims into the top-of-stack
   `argv`/`envp` string area. Loading the two libraries by their natural paths
   made `argv[1]` differ by 15 bytes, shifting that area by exactly 15 and
   changing the printed bytes. Fixed by copying both `.so`s to **equal-length
   paths** and using **identical `argv` and environment** for both children.
2. **ASLR.** When the residue *is* a stack address, its bytes are the
   randomized address itself, so two processes disagree by construction. Fixed
   by disabling randomization in the child (`personality(ADDR_NO_RANDOMIZE)` via
   `pre_exec`) so both children get an identical stack layout.

Each row is executed as: spawn one child per library, identical argv/env, ASLR
off, library loaded via `libloading` and called **only** through its exported
symbols; compare the children's raw stdout bytes. Randomized inputs use a fixed
seed (SplitMix64, seed `0x5EED_1234_ABCD_F00D`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | LAZY; empty string `""` | [x] |
| 2 | `printLine` | LAZY; single byte `"a"` | [x] |
| 3 | `printLine` | LAZY; short ASCII `"string"` | [x] |
| 4 | `printLine` | LAZY; 64 randomized printable strings, lengths 0–255 (fixed seed) | [x] |
| 5 | `printLine` | LAZY; all 255 non-NUL byte values, one per call | [x] |
| 6 | `printLine` | LAZY; embedded `\n` (`"a\nb\nc"`) — `puts` adds one more | [x] |
| 7 | `printLine` | LAZY; interior NUL (`"ab\0cd"`) — output truncates at NUL | [x] |
| 8 | `printLine` | LAZY; oversized: 1 MiB of `'x'` | [x] |
| 9 | `printLine` | NOW; same shapes as rows 1–8 | [x] |
| 10 | `printLine` ×N | LAZY; repeated calls in one process (buffering / ordering) | [x] |
| 11 | `good` | LAZY; fresh call — prints `string\n` | [x] |
| 12 | `good` | NOW; fresh call | [x] |
| 13 | `good` ×3 | LAZY; repeated, verifying identical output each time | [x] |
| 14 | `bad` | LAZY; **fresh/unprimed** call — residue written by `dlopen` (see *Attribution* below) | [x] |
| 15 | `bad` | NOW; fresh/unprimed call (no resolver run) | [x] |
| 16 | `bad` | LAZY; **primed by `good()`** — reads back the `"string"` pointer `good` stored in the shared slot; pinned to the absolute value `string\nstring\n` | [x] |
| 17 | `bad` | NOW; primed by `good()` | [x] |
| 18 | `bad` | LAZY; **primed by `printLine(s)`** — reads back the spilled parameter, for `s` in {`""`,`"a"`,`"foo"`,`"abcdefgh"`,`NULL`}; each pinned absolutely | [x] |
| 19 | `bad` ×2 | LAZY; repeated call, same depth (unprimed + primed) | [x] |
| 20 | `bad` ×2 | NOW; repeated call (unprimed + primed) | [x] |
| 21 | `driver` | LAZY; `useGood = 1` → `good()` | [x] |
| 22 | `driver` | NOW; `useGood = 1` | [x] |
| 23 | `driver` | LAZY; `useGood = 0` → `bad()`, **first** call (resolver clobbers the slot) | [x] |
| 24 | `driver` | NOW; `useGood = 0`, first call (resolver does **not** run) | [x] |
| 25 | `driver` ×2 | LAZY; `0` then `0` — first vs subsequent differ (A6) | [x] |
| 26 | `driver` ×2 | NOW; `0` then `0` | [x] |
| 27 | `driver` | LAZY; boundary non-zero values `-1`, `2`, `INT_MIN`, `INT_MAX` | [x] |
| 28 | `driver` | NOW; boundary non-zero values `-1`, `2`, `INT_MIN`, `INT_MAX` | [x] |
| 29 | `driver` | LAZY; 64 randomized non-zero `int`s (fixed seed) → all take `good()` | [x] |
| 30 | `driver` | LAZY; `1` then `0` — `good()` primes the slot, then `bad()` reads it | [x] |
| 31 | `driver` | NOW; `1` then `0` | [x] |
| 32 | `driver` | LAZY; `0` then `1` — reverse order | [x] |
| 33 | mixed `driver`/`bad`/`good`/`printLine` | LAZY; 40 randomized op sequences of length 1–8 over a dirtied stack (fixed seed) | [x] |
| 34 | mixed `driver`/`bad`/`good`/`printLine` | NOW; 40 randomized op sequences of length 1–8 (fixed seed) | [x] |
| 35 | `bad` interleaved with `good` | LAZY; `g b g b g b` — slot alternately primed and read (primed form pinned to six `string` lines) | [x] |
| 36 | `driver` deep sequence | NOW; `d1 d1 d0 d0 d1 d0` — cross-branch residue carry-over | [x] |
| 37 | mixed, with **unprimed** `bad` | LAZY; 40 randomized sequences containing direct `b` (row 33b) — compared modulo loader-residue lines | [x] |

## Attribution: which `bad()` output is the translation's responsibility

`bad()` forwards an uninitialized pointer, so its output is decided by whatever
last wrote its `-0x8(%rbp)` slot. That splits into two provenances, and only one
of them is a property of the translated code:

**Library-determined (must be byte-identical; asserted exactly).** The slot was
last written by a preceding *library* call — `good()`'s store, `printLine()`'s
parameter spill, `driver()`'s frame plus the lazy-PLT resolver. Rows 16–18, 21–36
and the `tg`/`tp:`/`tn` trampoline rows are of this kind. They pass, and several
are pinned to absolute expected byte strings rather than only mirrored against C
(`good(); bad()` → `string\nstring\n`; `printLine("foo"); bad()` → `foo\nfoo\n`;
`printLine(NULL); bad()` → zero bytes).

**Loader-determined (not attributable to any translation).** On an *unprimed*
direct `bad()` call the slot was last written by `dlopen`, so the forwarded
pointer is a function of the loaded object's dependency list, `.dynsym` size and
segment sizes. `tests/phase_b_residue_control.rs` establishes this mechanically
inside this same harness: two **C** builds that are behaviourally identical by
construction — `c_src/src/driver.c` as shipped versus the same source plus 2000
unused exported functions and one extra `DT_NEEDED` — disagree on **6 of 6**
direct-`bad()` rows, while agreeing on all 10 library-residue rows. The
as-shipped C build prints `5b 11 f8 f7 ff 7f 0a` and the padded one
`4b e7 f6 f7 ff 7f 0a`. No Rust translation can match a value that a mere
recompilation of the C already changes.

Those rows are therefore compared with
`Fixture::assert_same_except_loader_residue`, which still requires the line
count to agree (so both sides took the same NULL-vs-non-NULL branch the same
number of times) and requires every line outside the residue lines to be
byte-identical. Reproduce with `scripts/residue_control.sh`.

## Suite sensitivity

A differential suite that cannot fail proves nothing, so sensitivity is checked
explicitly by `scripts/mutation_check.sh`. It substitutes deliberately wrong C
builds for the Rust side; each must be caught:

| mutant (one-line change to a copy of `driver.c`) | failing tests |
|---|---|
| `good()` stores `"strinG"` instead of `"string"` | 22 |
| `driver()` inverts its `if (useGood)` test | 16 |
| `printLine()` inverts its NULL guard | 37 |
| *(unmutated Rust `.so`)* | **0** |

An earlier iteration of this harness silently captured zero bytes for every case
(the child-process test filter did not match its own test path), which made all
36 rows pass vacuously. A second iteration named fixture directories by PID
alone, so tests running in parallel threads clobbered each other's copies and
the suite failed only under the default (parallel) test runner. Both are fixed:
a missing capture file is now a hard error, each fixture gets a unique
directory, and the control tests take explicit library paths instead of mutating
process-global environment variables. The absolute-value assertions above exist
so that a comparison-only suite can never again pass by comparing nothing to
nothing. The full suite is run repeatedly in parallel as a race check.

## Build-profile constraint

The verified artifact is the **release** cdylib (`cargo build --release`), which
is what `Cargo.toml`'s `[profile.release] panic = "abort"` and the task's build
instructions specify. Re-running the suite against the *debug* cdylib splits
perfectly along the uninitialized read:

* all 18 rows with no uninitialized read pass (every `printLine`, `good`, and
  `driver(non-zero)` row) — so all **defined** behaviour is byte-identical in
  both profiles;
* all 19 rows that observe the CWE-457 read fail, because at `opt-level=0` the
  local area sits at a different frame offset, so `bad()` reads a different word
  than the C's `-0x8(%rbp)`.

This is expected: reproducing the C's frame geometry is what makes the UB
observable-equal, and that geometry is tied to the optimized codegen the crate
is configured for (`-Cforce-frame-pointers=yes` in `.cargo/config.toml`,
`-z lazy` from `build.rs`). It is recorded here rather than hidden.

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so the cross-product of
feature combinations is the single default build. Every row above is therefore
already the complete feature matrix. `scripts/all_features.sh` extracts the
feature list from `Cargo.toml` (rather than hard-coding it, so it stays correct
if features are added) and runs the full suite under `<default>`,
`--no-default-features` and `--all-features`; all three pass 61/61.

## Result

All 37 rows pass. `cargo test` → 38 + 4 + 15 + 4 = **61 tests, 0 failures**,
under each of the 3 feature combinations. Tests live in:

| file | phase | tests |
|---|---|---|
| `tests/phase_b_configs.rs` | B — valid paths, one test per `CONFIGS.md` row | 38 |
| `tests/phase_c_errors.rs` | C — one test per `ERRORS.md` row + generic boundaries | 15 |
| `tests/phase_d_symbols.rs` | D — `nm -D` parity, no unresolved symbols, dlsym-callable | 4 |
| `tests/phase_b_residue_control.rs` | attribution controls + exact slot-fidelity | 4 |
| `tests/common/mod.rs` | shared child-process differential harness | — |
