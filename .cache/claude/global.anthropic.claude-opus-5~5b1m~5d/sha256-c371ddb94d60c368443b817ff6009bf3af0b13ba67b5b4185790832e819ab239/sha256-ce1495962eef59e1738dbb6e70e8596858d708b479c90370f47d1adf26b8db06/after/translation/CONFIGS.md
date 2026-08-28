# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Axes the C actually branches on

There are no runtime options, no flags, no modes, no `#ifdef`s and no global
state in `c_src/` — `grep -c '#if\|#ifdef\|static\|extern' c_src/src/lib.c` finds
none of them. The single public entry point is also the lowest-level one:

| entry point | signature | it *is* the low-level API (no wrappers exist) |
|-------------|-----------|-----------------------------------------------|
| `wcscat` | `int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src)` | yes — `nm -D` exports exactly this one symbol |

So the configuration surface is entirely **input shape**. The branches the C
takes are (line numbers from `c_src/src/lib.c`):

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A. `dst` pointer | NULL / non-NULL | L7 |
| B. `numElem` | `0` / `1` / `2` / small `n` / huge-but-safe / overflowing (`dst+numElem` wraps) | L7, L13, L15 |
| C. `src` pointer | NULL / non-NULL | L9 |
| D. `dst` window content | `dst[0]==0` (empty) / `0` at index `k` with `0<k<numElem` / no `0` in `[0,numElem)` (full) / `0` exactly at `numElem-1` | L13 |
| E. `src` content | `src[0]==0` (empty) / length `L>=1` | L16 |
| F. fit relation `k + L + 1` vs `numElem` | `<` (room to spare) / `==` (exact fit) / `== numElem+1` (off by one) / `>` (well over) | L15/L16/L19 |
| G. `wchar_t` payload values | `0` only terminates; every other `i32` (incl. negatives, `i32::MIN/MAX`, surrogates, >U+10FFFF) is an ordinary char | L13, L16 (`!= 0` / `== 0` are the *only* value tests) |
| H. tail of `dst` beyond the write | must be preserved bit-exactly (the C never touches it) | absence of any write past `ptr` |
| I. `src`/`dst` aliasing | `src` disjoint / `src` points inside `dst` (forward element-by-element copy) | L16 |

Rows below are the cross-product of A–I pruned to combinations the C treats
differently. Rows that are *rejections* live in `ERRORS.md`; this table is the
valid/accepted-input mirror, plus the shape combinations that reach a rejection
through a *different code path* than the ones already tabulated there.

Every row is driven with **many randomized inputs** (fixed-seed PRNG, seed
`0x5EED_C0DE_1234_5678`), not a single hand-picked value: random `numElem`,
random `k`, random `L`, random non-zero `wchar_t` fill (including negatives),
random guard fill, and full-buffer bit-comparison including a guard region on
both sides of the window.

## Row → test mapping

Row `N` is covered by the test function `cfgNN_*` in
`tests/phase_b_valid.rs` (e.g. row 9 → `cfg09_prefix_room_to_spare`). Each such
test additionally cross-checks the observed buffer against an independent
re-derivation of the C semantics (`fn model` in that file), so a row cannot pass
by both implementations being wrong in the same way.

Rows 15/17/19 are additionally backed by hardware: `tests/phase_d_bounds.rs`
places the buffers flush against `PROT_NONE` guard pages, so any access outside
`[dst, dst+numElem)` — or past `src`'s terminator — faults instead of silently
producing the right value.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `wcscat` | `numElem == 1`, `dst[0] == 0` (empty), `src` empty (`src[0]==0`) → exact fit in the smallest legal window | [x] |
| 2 | `wcscat` | `numElem == 1`, `dst[0] == 0`, `src` non-empty → single-slot truncation path | [x] |
| 3 | `wcscat` | `numElem == 2`, all four `dst`-empty/`dst`-full × `src`-empty/`src`-non-empty combinations | [x] |
| 4 | `wcscat` | empty `dst` (`k==0`), random `L` with `L + 1 < numElem` → append into empty buffer, room to spare | [x] |
| 5 | `wcscat` | empty `dst` (`k==0`), `L + 1 == numElem` → exact fit, last slot gets the NUL | [x] |
| 6 | `wcscat` | empty `dst` (`k==0`), `L == numElem` → off-by-one, truncation | [x] |
| 7 | `wcscat` | empty `dst` (`k==0`), `L >> numElem` (src far longer than the window) | [x] |
| 8 | `wcscat` | non-empty `dst` prefix `0 < k < numElem-1`, `src` empty → writes a single NUL at `dst[k]`, prefix preserved | [x] |
| 9 | `wcscat` | non-empty `dst` prefix `k`, random `L`, `k + L + 1 < numElem` → normal concatenation, room to spare | [x] |
| 10 | `wcscat` | non-empty `dst` prefix `k`, `k + L + 1 == numElem` → exact fit | [x] |
| 11 | `wcscat` | non-empty `dst` prefix `k`, `k + L == numElem` → off-by-one truncation (payload fits, terminator does not) | [x] |
| 12 | `wcscat` | non-empty `dst` prefix `k`, `k + L > numElem` → truncation with partial copy, then `dst[0]=0` clobber | [x] |
| 13 | `wcscat` | `dst` terminator exactly at `numElem-1` (`k == numElem-1`, one free slot), `src` empty → success, NUL written into the last slot | [x] |
| 14 | `wcscat` | `dst` terminator exactly at `numElem-1`, `src` non-empty → truncation, one char written then `dst[0]=0` | [x] |
| 15 | `wcscat` | `dst` completely full/unterminated in `[0,numElem)` → scan loop exhausts, `src` never read, `ret 34`, `dst[0]=0`, tail preserved | [x] |
| 16 | `wcscat` | `dst` has its `0` *outside* the window (at index `>= numElem`) → still "full", same path as row 15 | [x] |
| 17 | `wcscat` | `numElem` smaller than the real allocation, with a guard region after the window that must stay bit-identical (no over-write past `numElem`) | [x] |
| 18 | `wcscat` | `numElem` huge but non-overflowing (`1<<40`, `1<<48`) with `dst` terminated early and `src` fitting inside the real allocation → success | [x] |
| 19 | `wcscat` | `numElem` chosen so `dst + numElem` overflows (`SIZE_MAX`, `SIZE_MAX/4`, `SIZE_MAX/2`, `1<<62`) → both loops skipped, `ret 34`, `dst[0]=0` | [x] |
| 20 | `wcscat` | extreme/negative `wchar_t` payloads in `src` and in the `dst` prefix (`i32::MIN`, `-1`, `i32::MAX`, `0xD800`, `0x110000`, `0x41424344`) — none may be mistaken for a terminator | [x] |
| 21 | `wcscat` | `src` aliases `dst`: `src == dst.add(off)` for `off` inside the prefix (self-append), forward copy order | [x] |
| 22 | `wcscat` | `src == dst` exactly (degenerate self-append) | [x] |
| 23 | `wcscat` | `src` aliases the *tail* of `dst` beyond the window (`src == dst.add(numElem)`) → reads the region the window may not write | [x] |
| 24 | `wcscat` | idempotence/sequencing: two `wcscat` calls in a row on the same buffer (append twice), the shape a real consumer produces | [x] |
| 25 | `wcscat` | repeated append until the buffer saturates (loop until `ret != 0`), checking the whole state trajectory call-by-call | [x] |
| 26 | `wcscat` | full randomized fuzz over the whole axis cross-product: random `dst` allocation size, random `numElem <= alloc`, random `k` (incl. "no terminator"), random `L`, random payloads, random guards — 200 000 cases | [x] |
| 27 | `wcscat` | randomized fuzz with `numElem > alloc` restricted to cases where `dst` is terminated inside the allocation (so the C stays in bounds) | [x] |
| 28 | `wcscat` | return-code domain closure: over the whole fuzz corpus, both implementations only ever return `{0, 22, 34}` and always return the *same* one | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so the cross-product of
features is a single point. Phases B and C are nevertheless re-run under
`--no-default-features`, `--all-features` and the default, by script, to prove it.

| combo | status |
|-------|--------|
| (default) | [x] |
| `--no-default-features` | [x] |
| `--all-features` | [x] |
