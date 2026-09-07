# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C source actually takes.

## Public surface (all entry points)

`c_src/include/lib.h` declares exactly one function:

```c
int pinflate(void *in, int in_bytes, void *out, int out_bytes);
```

but the `.so` also exports **seven mutable data objects** that are part of the
ABI and that `pinflate` reads on every call. They are therefore genuine
low-level entry points and are exercised as such (rows 41–46):

`cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`,
`cp_dist_extra_bits`, `cp_dist_base` (inputs) and `cp_error_reason` (output).

There is **no binary/driver target** in either project (`CMakeLists.txt` builds
only `add_library(... SHARED)`, `Cargo.toml` declares only
`crate-type = ["cdylib"]`), so the "compare stdout of the C and Rust binaries"
sub-item of Phase B is not applicable.

## Axes the C branches on

| axis | values the C distinguishes | source |
|---|---|---|
| A. `btype` | 0 stored / 1 fixed / 2 dynamic (3 = error, see ERRORS.md) | `pinflate` `switch (btype)` |
| B. `bfinal` | 1 (single block) / 0 (`do{}while(!bfinal)` loops → multi-block stream) | `pinflate` |
| C. input pointer alignment `first_bytes = ((in+3)&~3)-in` | 0, 1, 2, 3 — seeds `s->bits` from leading bytes and shifts `words`/`word_count` | `pinflate` |
| D. `last_bytes = (in_bytes-first_bytes)&3` | 0 (`final_word_available = 0`) / 1 / 2 / 3 | `pinflate` |
| E. `word_count = (in_bytes-first_bytes)/4` | 0 (no 32-bit refills at all) / ≥1 | `pinflate`, `cp_peak_bits` |
| F. `cp_peak_bits` refill branch | no refill (`count >= n`) / word refill / final-word refill | `cp_peak_bits` |
| G. `cp_build`'s `s` argument | non-NULL (also fills `s->lookup`) / NULL | `cp_fixed`, `cp_dynamic` |
| H. code length in `cp_build` | 0 (skipped) / 1..9 (`lookup` filled) / 10..14 (no `lookup`) / 15 (**excluded from the returned `first[15]`** — original quirk) | `cp_build` |
| I. `cp_block` symbol class | `<256` literal / `==256` end-of-block / `>256` length-distance | `cp_block` |
| J. length symbol 257..285 → `cp_len_extra_bits` | 0, 1, 2, 3, 4, 5 extra bits; 285 = base 258 / 0 extra | `cp_block` |
| K. length symbols 286, 287 (present in the fixed tree, no DEFLATE meaning) | `cp_len_extra_bits[29..30] = 0`, `cp_len_base[29..30] = 0` ⇒ length 0 | `cp_block` |
| L. distance symbol 0..29 → `cp_dist_extra_bits` | 0,1,2,…,13 extra bits | `cp_block` |
| M. `backwards_distance` | `== 1` → `memset` path / `!= 1` → byte-copy loop (incl. overlapping `distance < length`) | `cp_block` `switch` |
| N. distance symbols 30, 31 (dynamic tree only) | `cp_dist_base[30..31] = 0` ⇒ distance 0 ⇒ `src == dst` self-copy | `cp_block` |
| O. `cp_dynamic` HLIT | `nlit = 257 + read(5)` → 257 … 288 | `cp_dynamic` |
| P. `cp_dynamic` HDIST | `ndst = 1 + read(5)` → 1 … 32 | `cp_dynamic` |
| Q. `cp_dynamic` HCLEN | `nlen = 4 + read(4)` → 4 … 19 (drives `cp_permutation_order` prefix) | `cp_dynamic` |
| R. `cp_dynamic` code-length opcode | 0..15 literal / 16 copy-previous 3..6 / 17 zeros 3..10 / 18 zeros 11..138 | `cp_dynamic` `switch (sym)` |
| S. stored `LEN` | 0 / 1 / 2 / 3 / 4 (sub-word) / large (multi-word); `in_bytes` must equal `5 + LEN` for the `bits_left/8 <= LEN` check to pass | `cp_stored` |
| T. `out_bytes` | exactly the produced size / larger than needed | `cp_block` bound checks |
| U. produced output size | 0 bytes / 1 byte / many | — |
| V. exported table contents mutated by the caller | default / patched | the 6 `D` data symbols |

## Rows (pruned cross-product — each row is a combination the C treats differently)

Every row is driven with **many randomized inputs** (fixed seed `0x5EED_1234`,
≥ 32 samples/row unless the row is a single degenerate shape), and both
libraries are called through their `.so` exports. A row passes only when
return value, the full `out` buffer bytes, and the `cp_error_reason` string
match byte-for-byte for every sample.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `pinflate` | A=0 stored, B=final, S: LEN=0, C=0, T exact | [x] |
| 2 | `pinflate` | A=0 stored, B=final, S: LEN=1, C=0 | [x] |
| 3 | `pinflate` | A=0 stored, B=final, S: LEN=2,3,4 (sub-word), C=0 | [x] |
| 4 | `pinflate` | A=0 stored, B=final, S: LEN random 5..4096 (multi-word), C=0 | [x] |
| 5 | `pinflate` | A=0 stored, B=final, C=1,2,3 (misaligned `in`) × D all residues | [x] |
| 6 | `pinflate` | A=0 stored, B=final, T: `out_bytes` > LEN | [x] |

> **Rows 1-6 are differential only.** The C's `cp_ptr` does **not** point at the
> stored payload for most shapes: the final-word refill adds `bits_left` to
> `count`, which over-counts the bits it actually supplied, so
> `(char *)(words + word_index) - count/8` lands short of the payload. For
> `LEN == 1` the C copies the high byte of the `LEN` field instead of the
> payload. This is ground truth, so these rows assert only that both libraries
> agree (plus `ret == 1`), and row 4 checks the decoded bytes only for the
> `len % 4 == 1` shapes where `cp_ptr` does land correctly.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 7 | `pinflate` | A=1 fixed, single literal 0..143 (8-bit code), U=1 byte — all 144 exhaustively | [x] |
| 8 | `pinflate` | A=1 fixed, single literal 144..255 (9-bit code) — all 112 exhaustively | [x] |
| 9 | `pinflate` | A=1 fixed, random literal runs (both code widths), U=many | [x] |
| 10 | `pinflate` | A=1 fixed, empty block (only end-of-block sym 256), U=0 | [x] |
| 11 | `pinflate` | A=1 fixed, I=length/dist, J: every length symbol 257..285 × every extra-bit value, M: distance ≠ 1 | [x] |
| 12 | `pinflate` | A=1 fixed, M: `backwards_distance == 1` (memset path), lengths 3..258 | [x] |
| 13 | `pinflate` | A=1 fixed, overlapping match `1 < distance < length` (byte-copy loop) | [x] |
| 14 | `pinflate` | A=1 fixed, L: every distance symbol 0..29 (extra bits 0..13), distances up to 32768 | [x] |
| 15 | `pinflate` | A=1 fixed, K: length symbols 286/287 ⇒ length 0, base 0 | [x] |
| 16 | `pinflate` | A=1 fixed, C=1,2,3 misaligned input × D all residues | [x] |
| 17 | `pinflate` | A=1 fixed, E=0 (`word_count == 0`: whole stream fits in `first_bytes` + final word) | [x] |
| 18 | `pinflate` | A=1 fixed, F: forced final-word refill (D≠0) mid-symbol | [x] |
| 19 | `pinflate` | A=1 fixed, T: `out_bytes` exactly equals produced size | [x] |
| 20 | `pinflate` | A=1 fixed, randomized literal+match mixture, 256 samples | [x] |
| 21 | `pinflate` | A=2 dynamic, O: HLIT 257 (minimum) | [x] |
| 22 | `pinflate` | A=2 dynamic, O: HLIT 288 (maximum), incl. length symbols 285/287 | [x] |
| 23 | `pinflate` | A=2 dynamic, P: HDIST 1 (minimum) ⇒ distance 1 ⇒ memset path | [x] |
| 24 | `pinflate` | A=2 dynamic, P: HDIST 32 (maximum), 16 distance symbols | [x] |
| 25 | `pinflate` | A=2 dynamic, Q: HCLEN 4 (minimum). Only `cp_permutation_order[0..4] = {16,17,18,0}` is transmittable, so every code length must be 0 ⇒ empty literal tree ⇒ `cp_decode` reads `tree[-1]`. Differential only | [x] |
| 26 | `pinflate` | A=2 dynamic, Q: HCLEN 19 (maximum, full `cp_permutation_order`) | [x] |
| 27 | `pinflate` | A=2 dynamic, Q: HCLEN 5..19 (15 distinct values) | [x] |
| 28 | `pinflate` | A=2 dynamic, R: opcode 16 (copy-previous, repeat 3..6) | [x] |
| 29 | `pinflate` | A=2 dynamic, R: opcode 17 (zeros 3..10) | [x] |
| 30 | `pinflate` | A=2 dynamic, R: opcode 18 (zeros 11..138) | [x] |
| 30b | `pinflate` | A=2 dynamic, R: opcodes 16+17+18 together in one block | [x] |
| 31 | `pinflate` | A=2 dynamic, H: code lengths 1..9 only (`lookup` fully populated) | [x] |
| 32 | `pinflate` | A=2 dynamic, H: maximum code length 10,11,12,13,14 (`lookup` skipped for the long codes) | [x] |
| 33 | `pinflate` | A=2 dynamic, N: distance symbols 30/31 ⇒ `cp_dist_base == 0` ⇒ distance 0 ⇒ `src == dst` self-copy. Differential only | [x] |
| 34 | `pinflate` | A=2 dynamic, C=1,2,3 misaligned × D residues | [x] |
| 35 | `pinflate` | A=2 dynamic, randomized trees (random symbol sets, shuffled length multisets) + randomized payloads, 256 samples | [x] |
| 36 | `pinflate` | A=2/1/0, streams produced by a real DEFLATE encoder (`flate2`, levels 0..9) over 6 data shapes × 4 alignments = 240 streams | [x] |
| 37 | `pinflate` | B=0: multi-block stream, two fixed blocks | [x] |
| 38 | `pinflate` | B=0: multi-block stream, fixed → dynamic → fixed | [x] |
| 39 | `pinflate` | B=0: multi-block, last block stored (the only legal position). Differential only, same `cp_ptr` caveat as rows 1-6 | [x] |
| 40 | `pinflate` | B=0: multi-block with cross-block back-references | [x] |
| 41 | data symbols | V: all 6 input tables compared byte-for-byte between the two `.so`s **and** against the literals in `c_src/src/lib.c`; `cp_error_reason` NULL in both | [x] |
| 42 | data symbols + `pinflate` | V: patch `cp_len_base[0..29]` identically in both, run a fixed-block match stream | [x] |
| 43 | data symbols + `pinflate` | V: patch `cp_len_extra_bits` identically in both, run a fixed-block match stream | [x] |
| 44 | data symbols + `pinflate` | V: patch `cp_dist_base` + `cp_dist_extra_bits` identically, run a match stream | [x] |
| 45 | data symbols + `pinflate` | V: patch `cp_permutation_order` (random transposition) identically and write the dynamic block with the *same* permutation, so the stream stays self-consistent and must still decode correctly | [x] |
| 46 | data symbols + `pinflate` | V: patch `cp_fixed_table[0..288]` with a shuffled copy of its own length multiset (still a complete code) and encode against the patched table — proves the Rust actually reads its exported global | [x] |
| 47 | `pinflate` | H=15: tree containing length-15 codes, hitting the `first[15]` quirk (`cp_build` returns `first[15]`, which **excludes** the length-15 symbols). Differential only | [x] |
| 48 | `pinflate` | G: `cp_build(s,…)` (fills `lookup`) vs `cp_build(0,…)` — a dynamic block with max code length 9 (all `lookup` entries written) and one with 12 (long codes skipped) | [x] |

## Excluded (no defined behaviour to match)

* `cp_len_extra_bits[symbol]` / `cp_len_base[symbol]` for `symbol > 30` and
  `cp_dist_*[symbol]` for `symbol > 31`. Reachable only when `cp_decode` reads
  `tree[-1]` (i.e. `hi == 0`) *and* the resulting garbage `key` happens to
  satisfy the `lib.c:217` assert, which requires deliberately seeding
  `s->lookup` from a previous block. The two `.so`s place their six exported
  tables in a different order in `.data`, so the out-of-bounds bytes differ by
  construction; matching would mean pinning the Rust data-section layout to the
  C's, which is not part of the C's interface. Not reached by any of the
  ~125 000 fuzz cases run. The in-bounds neighbours **are** covered: length
  symbols 286/287 (rows 15) and distance symbols 30/31 (row 33).
* `in_bytes` larger than the caller's real allocation (unmapped reads).
* `calloc` failure (ERRORS.md row 17): the C ignores the NULL and faults; Rust's
  global allocator aborts instead.
* `cp_dynamic`'s `lens[]` overflow reaching past the modelled stack frame. `n`
  is reset as soon as a write lands on it (frame offset `-0x8`, `lens[376]`), so
  `n ≤ 379` and every reachable index lies inside the modelled frame; but a
  `nlit` corrupted to a very large value would make the final `cp_build` read
  beyond the frame, where the C reads its caller's stack and the Rust reads
  zeroed slack.

## Modelled, not excluded

Two pieces of the C's undefined behaviour turned out to be *observable* and are
reproduced deliberately (see the long comments in `src/lib.rs`):

* **`cp_dynamic`'s stack frame.** The repeat opcodes overflow `lens[288+32]`
  into `lenlens`, `sym`, `nlen`, `ndst`, `nlit`, the three loop counters and
  finally `n`. Rust reproduces gcc's frame layout (verified against
  `objdump -d`) so the aliasing matches, including gcc's ordering of
  `lens[n++] = sym` (the increment is stored *before* the byte write).
  `lens[-1]`, read by opcode 16 when `n == 0`, aliases the top byte of the
  spilled `cp_state_t *` — zero for any canonical x86-64 address.
* **`cp_decode`'s `tree[-1]`.** `cp_state_t` is `#[repr(C)]` with the original
  field order, so the read lands on the same neighbouring member as in C
  (`lookup` for `lit`, `lit` for `dst`, `dst` for `len`), and the tree pointers
  are derived from the struct base so the read keeps valid provenance.
  Also: `32 - (key & 0xF)` can be 32, which gcc compiles to `shr %cl` with the
  count masked to 5 bits; Rust reproduces that with `& 31`.

`cp_build`'s `counts[16]` overflow (indexed by a code length that may exceed 15)
needs no frame model: whenever it happens, the `assert(len < 16)` in the second
loop aborts, and the overflowing stores can only reach `first`, `codes` and the
loop counters — never `sym_count` (at `-0xfc`, *below* `counts` at `-0xe0`), so
the second loop always runs its full range and always aborts. Both libraries
therefore end in `SIGABRT` (confirmed by `errors.rs::row15_...`).

## Test-harness notes

* Every call runs in a `fork()`ed child, so `abort()` is a comparable outcome.
  Fatal signals are turned into `_exit(128 + signo)` by a handler in the child
  because this host pipes `core_pattern` to `systemd-coredump` (~380 ms per
  abort, not suppressed by `RLIMIT_CORE = 0`).
* A 40 ms `ITIMER_REAL` watchdog makes non-termination (ERRORS.md row 25) an
  observable, comparable outcome.
* Both buffers get 64 KiB zeroed guard regions, because `cp_stored`'s `memcpy`
  is bounded by neither. The out guard is compared via an FNV-1a hash so a
  stray write out there is still caught without shipping 64 KiB per case.
* Both `.so`s are loaded with `RTLD_NOW | RTLD_LOCAL`, so their identically
  named globals stay separate and every PLT entry is bound before the fork.

