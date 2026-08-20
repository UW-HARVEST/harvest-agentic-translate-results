# CONFIGS.md — Phase B: configuration-surface table (valid inputs)

## Axes the C code actually branches on

Derived from `c_src/include/driver.h` (public header) and the `if`/`else`
statements in `c_src/src/driver.c`. There is no runtime option struct, no
init/config call, no global flag, no byte-order or width handling and no
`#ifdef`, so the axes are exactly:

**Axis 1 — entry point (full set, including the lowest-level ones).** The `.so`
exports 4 symbols. `driver` is the convenience/one-shot wrapper declared in the
header; `bad` and `good` are the mid-level entry points; `printLine` is the
lowest-level entry point and is the only one that takes data. All four are
tested directly, not just `driver`.

| level | entry point | signature |
|---|---|---|
| low | `printLine` | `void printLine(const char *line)` |
| mid | `bad` | `void bad(void)` |
| mid | `good` | `void good(void)` |
| top | `driver` | `void driver(int useGood)` |

**Axis 2 — `printLine`'s pointer/payload shape** (`driver.c:30` branches on
NULL; `puts` then walks bytes to the terminator): NULL vs non-NULL; length 0 /
1 / many; bytes ASCII vs high (`0x80..=0xFF`) vs control (`\n`, `\t`, `\r`);
length relative to libc's 4096-byte `stdout` buffer (below / exactly at /
across); `%`-containing payloads; allocation with no slack after the terminator.

**Axis 3 — `driver`'s `useGood` truthiness** (`driver.c:60`): zero vs non-zero,
where non-zero must be evaluated over the whole 32-bit `int` (so `0x100`,
`0x10000`, `INT_MIN` are truthy even though their low byte is 0).

**Axis 4 — call count / sequencing.** `helperGood1`'s `charString` has *static*
storage duration, so its pointer must stay valid and identical across calls;
`stdout` is a shared, buffered stream, so accumulated output across a sequence
of mixed calls is itself observable state.

## Rows (pruned cross-product of the axes the C distinguishes)

Every row is exercised with **many randomized inputs (SplitMix64, fixed seed
`0x5EED_1234_ABCD_EF01`)** where the row has a data axis, and asserted
byte-for-byte between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `printLine` | non-NULL, length 0 (`""`) — guard passes, empty payload | [x] |
| C2 | `printLine` | non-NULL, length 1, every byte value `0x01..=0xFF` (255 cases, exhaustive) | [x] |
| C3 | `printLine` | randomized printable-ASCII payloads, length 1..=64, 512 iterations | [x] |
| C4 | `printLine` | randomized arbitrary non-NUL bytes `0x01..=0xFF` (non-UTF-8 included), length 1..=128, 512 iterations | [x] |
| C5 | `printLine` | randomized payloads containing embedded control bytes `\n \t \r \0`-adjacent, length 1..=64, 256 iterations | [x] |
| C6 | `printLine` | payloads containing `printf` format specifiers (`%s`, `%n`, `%d`, `%%`, `%1000000d`) — must be emitted verbatim | [x] |
| C7 | `printLine` | length exactly at/around the libc 4096-byte `stdout` buffer: 4094, 4095, 4096, 4097, 4098, 8191, 8192, 8193 | [x] |
| C8 | `printLine` | long randomized payloads, length 1..=20000, 64 iterations (crosses the buffer repeatedly) | [x] |
| C9 | `printLine` | terminator is the final byte of the allocation (no trailing slack), randomized lengths | [x] |
| C10 | `printLine` | same pointer passed N times in one capture (N randomized 2..=16) → N identical lines | [x] |
| C11 | `printLine` | randomized *sequence* of distinct payloads in one capture → concatenated stream | [x] |
| C12 | `good` | single call → `"helperGood1 string\n"` | [x] |
| C13 | `good` | N repeated calls (N randomized 2..=32) — static-storage pointer stability | [x] |
| C14 | `bad` | single call (`helperBad` → NULL → silent) | [x] |
| C15 | `bad` | N repeated calls | [x] |
| C16 | `driver` | `useGood = 0` → `bad()` branch | [x] |
| C17 | `driver` | `useGood = 1` → `good()` branch | [x] |
| C18 | `driver` | `useGood = -1` (all bits set) → truthy | [x] |
| C19 | `driver` | `useGood = INT_MAX`, `INT_MIN` → truthy | [x] |
| C20 | `driver` | `useGood` = low-byte-zero non-zero values `0x100`, `0x10000`, `0x1000000`, `0x7F00`, `-256` → truthy | [x] |
| C21 | `driver` | randomized `i32` over the full range, 2048 iterations (mixes zero and non-zero) | [x] |
| C22 | `driver` | N repeated calls with the same `useGood` (both branches) | [x] |
| C23 | `bad` + `good` | randomized interleaving of the two mid-level entry points in one capture | [x] |
| C24 | all 4 | randomized interleaving of `printLine`/`bad`/`good`/`driver` in one capture — the composed pipeline, 256 programs of 1..=24 steps | [x] |
| C25 | `driver` vs `good`/`bad` | equivalence of the wrapper against the mid-level calls it dispatches to: `driver(nz)` ≡ `good()`, `driver(0)` ≡ `bad()`, cross-checked C↔Rust *and* wrapper↔callee | [x] |
