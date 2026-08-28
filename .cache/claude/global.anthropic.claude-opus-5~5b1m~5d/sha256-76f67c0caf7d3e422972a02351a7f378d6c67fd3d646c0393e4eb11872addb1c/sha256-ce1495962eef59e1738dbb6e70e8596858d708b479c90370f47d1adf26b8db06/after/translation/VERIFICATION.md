# VERIFICATION.md — differential verification of the C→Rust translation

The C in `c_src/` is the ground truth.  Everything below compares the two shared
objects **through their exported symbols only**, loaded with `libloading` — the
Rust functions are never called directly, so the `#[no_mangle]` wrappers are part
of what is tested.

```
C    : c_src/build/libharvest-work-VVQnEx.so   (cmake, -O0, NDEBUG *not* defined)
Rust : translation/target/release/libconvert_pix_lib.so
```

## Artifacts

| file | what it is |
|------|------------|
| `SYMBOLS.md` | every `nm -D` symbol of the C object and its Rust counterpart, plus the `.data`-image note |
| `ERRORS.md`  | 37 rows: every rejection, every `assert`, every unchecked index/array overflow in the C |
| `CONFIGS.md` | 56 rows: every valid option/shape combination the C branches on |

## Test binaries

| file | what it covers | tests |
|------|----------------|-------|
| `tests/symbols.rs` | `nm -D` parity C ⊆ Rust; no unresolvable/undefined API symbol (`RTLD_NOW` dlopen) | 2 |
| `tests/globals.rs` | CONFIGS rows 1–7: the six exported tables byte-for-byte, `cp_error_reason` initially `NULL` | 7 |
| `tests/convert_pix.rs` | CONFIGS rows 8–13 + ERRORS rows 22–23 | 9 |
| `tests/inflate.rs` | CONFIGS rows 14–40, 45–47: stored / fixed / dynamic, every symbol bucket, alignments, multi-block, 64 KiB payload | 30 |
| `tests/tamper.rs` | CONFIGS rows 41–44: the writable globals swapped for other *valid* values | 1 (four scenarios, serialised — `dlopen` shares one mapping per process) |
| `tests/errors.rs` | ERRORS rows 1–6, 24, 27–29: every path that *returns* an error | 11 |
| `tests/aborts.rs` | ERRORS rows 11, 13, 14, 16, 18–20, 25, 26 + three randomised sweeps, each scenario run in a child process | 14 |
| `tests/oob_tables.rs` | ERRORS rows 30a & 31 / CONFIGS rows 48–49, 56: `cp_dist_*[287]` read via an empty distance tree; `cp_build`'s `counts[]` overrun | 4 |
| `tests/dynamic_overshoot.rs` | ERRORS rows 32–36 / CONFIGS rows 50–55: the `lens[288+32]` stack overrun, and the unchecked `lenlens[cp_permutation_order[i]]` index | 9 |

Total: **87 tests**, ~14 000 `cp_inflate`/`convert_pix` call pairs.

Every comparison checks, for both libraries:

* the return value,
* the **whole** output buffer including the over-allocated tail (so an overrun is
  compared, not hidden),
* the input buffer (so a stray write to it would be caught),
* the `cp_error_reason` **string** (not the pointer),
* and, for child-process scenarios, the exit code, the terminating signal and
  the exact `assert` diagnostic text on stderr.

## Divergences found and fixed

Matching symbols and green happy-path tests were *not* sufficient — three real
bugs only showed up after deriving `ERRORS.md`/`CONFIGS.md` from the C source:

1. **Out-of-bounds table reads followed the Rust link order, not the C's.**
   `cp_block` indexes `cp_len_extra_bits`/`cp_len_base` with `symbol - 257` and
   `cp_dist_extra_bits`/`cp_dist_base` with `distance_symbol`, unchecked.  A
   dynamic block may declare `HDIST` distance codes and give them all length 0,
   so `cp_build` returns 0 and `cp_decode(s, s->dst, 0)` reads `s->dst[-1] ==
   s->lit[287]`, whose symbol field is `287`.  The C then reads 255 entries past
   both arrays and gets zeros (past `.bss`, inside the last mapped page), so
   `backwards_distance == 0` and it returns `1`.  Rust's statics are in a
   different order with no padding, so it read a code length of `9`, consumed 9
   extra bits and returned `0`.
   *Fix:* `cp_data_byte()` / `cp_data_u32()` in `src/lib.rs` reconstruct the C's
   `.data`/`.bss` image at the offsets from `readelf`.
   *Test:* `tests/oob_tables.rs` (failed before, passes after).

2. **`cp_dynamic`'s RLE loop overran a local array into other locals.**
   `n < nlit + ndst` is only tested between symbols, but one symbol 16/17/18
   writes up to 138 entries, so the last run overshoots `uint8_t lens[288+32]`
   by up to 137 bytes.  `objdump -d` shows the -O0 frame exactly: those bytes
   land on `lenlens`, `sym`, `nlen`, **`ndst`**, **`nlit`**, the run counters,
   **`n`** and `i`.  Zeroing `ndst` empties the distance tree; zeroing `nlit`
   empties the literal tree (→ `SIGABRT`); zeroing the run counter makes it go
   negative while `n` is repeatedly reset, so the loop spins **for ever**.
   Rust had padded the array, so none of this happened.
   *Fix:* `struct DynFrame` in `src/lib.rs` models the frame explicitly, with
   every access to `lens`, `lenlens`, `nlit`, `ndst`, `n` and the counters going
   through it at the C's offsets.
   *Test:* `tests/dynamic_overshoot.rs::ov01..ov08`.

3. **`lenlens[cp_permutation_order[i]]` is also unchecked.**
   `cp_permutation_order` is exported and writable; an entry `> 18` makes the
   store land on `cp_dynamic`'s locals.  Slot `60` is the HCLEN loop counter `i`
   itself, and gcc's `++i` is a read-modify-write of the *stored* value, so the
   loop restarts and spins for ever.  The first version of `DynFrame` wrote
   `stale_i + 1` and so silently repaired the corruption.
   *Fix:* every `i` access is a fresh frame read-modify-write, in gcc's order
   (`cp_read_bits` → reload `i` → store → `++i`).
   *Test:* `tests/dynamic_overshoot.rs::ov09`.

## Behaviours deliberately reproduced (not "fixed")

* `cp_stored`'s bound check is `s->bits_left / 8 <= LEN`, the inverse of the
  intuitive one, so a stored block followed by any further input is rejected.
* `cp_stored` byte-aligns using `s->count & 7`, and `cp_ptr` recomputes the read
  position from `s->words + s->word_index - s->count / 8`.  Both are wrong once
  `cp_peak_bits` has taken its `final_word` branch, so a stored block with
  `LEN < 3` copies the *wrong bytes*, and a stored block after such a refill can
  drive `bits_left` negative and trip `cp_ptr`'s assert.
* `cp_stored` has no output bound check at all, so `LEN > out_bytes` overruns the
  output buffer.
* `cp_stored` never advances the reader past the payload, so a non-final stored
  block makes the next block header come out of the stored bytes.
* `cp_decode` reads `tree[-1]` for an empty tree, and `cp_decode`'s
  `search >> 32` / `key >> 32` rely on x86-64's shift-count masking.
* Failing `assert()`s abort the process; `src/lib.rs` reproduces glibc's exact
  `__assert_fail` message (progname, the absolute `__FILE__` cmake passes, line,
  `__PRETTY_FUNCTION__`, expression) so even the stderr text matches.

## Known limits of the reproduction

Three situations are undefined in C in a way no Rust program can mirror.  Each is
argued unreachable (or shown harmless) rather than merely ignored:

* **`lens[384..400]` (saved `%rbp` / return address).**  Unreachable: any run long
  enough to get there first zeroes the run counter at `lens[364..368]`, after
  which the loop never terminates and `n` cycles in `257..=376`.  Verified for
  every run length that could reach 384 (`tests/…::ov08`).
* **`lenlens[slot]` for `slot >= 64`** (i.e. `cp_permutation_order` deliberately
  overwritten with a value `>= 64`).  That writes into the *caller's* frame,
  which is outside the modelled frame.  Slots `19..=63` — everything that stays
  inside `cp_dynamic`'s own frame — are reproduced and tested (`…::ov09`).
* `calloc` returning `NULL` (`ERRORS.md` row 21): both implementations
  dereference the unchecked result, so both die with `SIGSEGV`, but this needs an
  allocator interposer to exercise.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]`, so there is exactly one
feature configuration.  It was verified under all of:

```sh
cargo test --offline --release
cargo test --offline --release --no-default-features
cargo test --offline --release --all-features
CP_RUST_SO=target/debug/libconvert_pix_lib.so cargo test --offline --release
```

The last one re-runs the whole suite against the **debug** cdylib, where
`overflow-checks` are on — the translation relies on wrap-around arithmetic in
many places, so this proves it does not panic where the C wraps.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation
cargo build --release
cargo test  --release -- --test-threads=1
```

`--test-threads=1` matters for `tests/tamper.rs` and `tests/dynamic_overshoot.rs`
(they mutate the shared `dlopen` mapping) and keeps the `fork()`-based harness
away from other threads' allocator locks.

Knobs: `CP_RUST_SO`, `CP_FUZZ_N`, `CP_FORK_FUZZ_N`, `CP_FORK_BOUND_N`,
`CP_CHILD_TIMEOUT_US`.
