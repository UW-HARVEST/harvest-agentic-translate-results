# CONFIGS.md — configuration surface (valid inputs)

Derived mechanically from `c_src/include/driver.h` and `c_src/src/driver.c`.

## Axes the C code actually branches on

**A. Public entry points.** The header declares only `driver`, but the `.so`
exports four global symbols (see `SYMBOLS.md`), and an external caller can
`dlsym` any of them. All four are therefore entry points, and the three
lower-level ones (`printLine`, `bad`, `good`) are exercised **directly**, not
only through the `driver` wrapper:

* `printLine(const char *)` — lowest level; the only function that touches stdio.
* `good()` / `bad()` — mid level; each builds a `char *` local and calls `printLine`.
* `driver(int)` — the one-shot convenience wrapper over `good`/`bad`.

**B. Runtime options/modes.** Exactly one: `driver`'s `useGood` `int`.
`if (useGood)` is a full 32-bit test against zero (`cmpl $0, -0x4(%rbp)`), so the
axis is *zero* vs. *any non-zero bit pattern*, not `0` vs `1`.

**C. Input shapes the code distinguishes.** `printLine`'s pointer is the only
data input:

* NULL vs non-NULL (the `if`), covered in `ERRORS.md`;
* string length: 0, 1, small, page-crossing, multi-MiB (`puts` internals switch
  on buffer size, so length is a real shape axis);
* byte content: ASCII, embedded `printf` specifiers, high/non-UTF-8 bytes, all
  255 non-NUL byte values, `\n`/`\r`/`\t`;
* the byte *after* the terminator must not be emitted (over-read shape);
* stdout destination shape: regular file vs pipe, because glibc picks
  full-buffering vs. its own sizing from `fstat` on fd 1 — both libraries share
  one `FILE *stdout` in the harness process, so this is an observable axis.

**D. Stack-residue shape — the axis unique to this library.** `bad()` reads an
uninitialized `char *` (CWE-457), so its output is a function of the caller's
stack state. The code distinguishes:

* residue `NULL` vs. residue = valid pointer;
* residue = pointer to strings of different lengths/contents;
* the depth the call is made from, and what ran immediately before
  (`good()` leaves a `"string"` pointer in exactly the slot a later `bad()`
  reads; `printLine` leaves its spilled parameter below);
* first call in the process vs. later calls — the C `.so` reaches `good`/`bad`
  from `driver` through lazy-bound PLT slots, so the *first* such call runs
  `_dl_runtime_resolve`, which overwrites the word `bad` is about to read.

Every row below pins axis D with a deterministic, harness-controlled residue
(the test writes a chosen 64-bit value over the relevant stack region before the
call, and calls both libraries through one identical call site at one identical
depth). Rows are driven with **many randomized inputs each, from a fixed seed**
(`SEED = 0x5EED_D1FF_2025_0901`, split-mix64), not one hand-picked value.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | randomized inputs | [x] |
|---|----------------|--------------------------------------------|-------------------|-----|
| 1 | `printLine` | non-NULL, length 1..=64, random printable ASCII | 4096 | [x] |
| 2 | `printLine` | non-NULL, length 0 (empty string) | 1 | [x] |
| 3 | `printLine` | non-NULL, random length 1..=4096, random bytes from the full 1..=255 range (non-UTF-8 included) | 2048 | [x] |
| 4 | `printLine` | non-NULL, each of the 255 single-byte strings `\x01`..`\xFF` individually | 255 | [x] |
| 5 | `printLine` | non-NULL, length exactly at/around glibc buffer boundaries: 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537 | 9 | [x] |
| 6 | `printLine` | non-NULL, 1 MiB and 4 MiB payloads (oversized shape) | 2 | [x] |
| 7 | `printLine` | non-NULL, buffer whose byte *after* the NUL terminator is random garbage (over-read shape) | 4096 | [x] |
| 8 | `printLine` | non-NULL, content containing `printf` conversion specifiers and `%n` | 16 | [x] |
| 9 | `printLine` | non-NULL, content containing `\n`, `\r`, `\t` at random positions | 2048 | [x] |
| 10 | `printLine` | stdout is a **pipe** (not a regular file), random payloads | 1024 | [x] |
| 11 | `good` | no options; called directly, fresh stack | 64 | [x] |
| 12 | `good` | called directly, residue pre-set to a random 64-bit-derived pointer (proves `good` ignores residue and always emits `string`) | 2048 + 64 indexed | [x] |
| 13 | `bad` | called directly, residue = `NULL` | 64 | [x] |
| 14 | `bad` | called directly, residue = pointer to a random string, length 1..=64 | 4096 | [x] |
| 15 | `bad` | called directly, residue = pointer to a random string, length 1..=4096, full byte range | 1024 | [x] |
| 16 | `bad` | called directly immediately after `good()` at the same depth (the `"string"`-left-in-the-slot interaction) | 256 × 3 shapes | [x] |
| 17 | `bad` | called directly immediately after `printLine(random)` at the same depth | 512 × 3 shapes | [x] |
| 18 | `driver` | `useGood` = 0 (selects `bad`), residue = `NULL`, both in a fresh process and with the PLT already bound | 64 + 64 | [x] |
| 19 | `driver` | `useGood` = 0, residue = pointer to a random string | 4096 | [x] |
| 20 | `driver` | `useGood` = 0, **first** `driver` call in the process (lazy-PLT / `_dl_runtime_resolve` shape) vs. a later one | 8 | [x] |
| 21 | `driver` | `useGood` = random non-zero 32-bit value (selects `good`) | 8192 | [x] |
| 22 | `driver` | `useGood` = random non-zero value with **zero low byte** (`v & !0xFF`, forced non-zero) | 4096 | [x] |
| 23 | `driver` | alternating `driver(1)` / `driver(0)` sequences of random length, random residue seeded once before the sequence (composed-pipeline shape) | 512 | [x] |
| 24 | `driver`, `good`, `bad`, `printLine` | random interleavings of all four entry points, random lengths, one shared pre-seeded residue — the full cross-product exercised as a real consumer would | 1024 | [x] |
| 25 | `printLine`, `bad` | residue / argument pointing to a string at a page boundary (last bytes of a mapped page, next page `PROT_NONE`) | 256 | [x] |
| 26 | `bad`, `driver(0)` | residue value swept over every 8-byte-aligned offset of the dirtied region (which stack word the C compiler actually reads) | 32 repeats × 64 offsets | [x] |

Observed for row 26: `bad()` reads `slot0060` and `driver(0)` reads `slot0056`
(once its PLT slot is bound) — the **same** word in both libraries.

## Two axes that are not input, but are configuration

* **Lazy PLT binding.** In a fresh process `driver`'s `call bad@plt` runs
  `_dl_runtime_resolve`, which overwrites the stack word `bad` then reads. So
  `driver(0)` behaves differently on its first call than on later ones, in both
  libraries. Rows 18, 20, 23 and 24 cover both states; rows that assert an
  *absolute* result for `driver(0)` do so only in the bound state.
* **Cargo profile.** The `.so`'s stack geometry must not depend on the
  optimization level. `scripts/verify.sh` runs the whole suite twice per feature
  combination — once against `target/release/libdriver.so` and once against
  `target/debug/libdriver.so`.

## Feature combinations

`Cargo.toml` has no `[features]` table and `src/lib.rs` contains no
`feature = ` cfg (verified: `grep -c feature src/lib.rs` → 0). The default
(empty) feature set is the only combination; `scripts/verify.sh` still runs both
`cargo test --release` and `cargo test --release --no-default-features` to prove
it. The remaining `cfg` axis is `target_arch = "x86_64"` (this host) vs. the
portable fallback.
