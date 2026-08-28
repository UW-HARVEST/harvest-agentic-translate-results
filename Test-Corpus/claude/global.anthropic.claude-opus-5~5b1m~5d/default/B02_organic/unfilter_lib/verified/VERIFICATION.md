# VERIFICATION.md — what was checked, how, and what was found

## How to reproduce

```bash
# C reference library, exactly as the task describes
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# everything, for every cargo feature combination and both cargo profiles
cd translation && ./run_all.sh          # ~12 min
cd translation && ./run_all.sh --quick  # ~3 min, skips the fuzz targets
```

`cargo test` alone also works: the harness builds whatever it needs (the C
reference, a `-DNDEBUG` build of the same C source, the stack-layout variant
used as the UB oracle, and the Rust cdylib with the running test binary's
feature set).

## Harness design (`tests/common/mod.rs`)

* **Both libraries are loaded with `libloading`** and driven only through their
  exported symbols — the `#[no_mangle] extern "C"` wrappers are part of what is
  under test. Nothing calls the Rust crate directly.
* Every case runs in a **`fork()`ed child** on a shared-memory scratch region:
  * a crash, an `abort()` from a live `assert()`, or an infinite loop in either
    library cannot take the runner down (and cannot be mistaken for a normal
    return);
  * the region is fully zeroed and prefilled identically before each of the two
    runs, and the input is placed at the same offset and the same 4-byte
    alignment, so the C code's deliberate over-reads/over-writes *around* the
    nominal buffers are comparable;
  * writes the child makes to the exported tables cannot leak into another case,
    which is what makes the "writable global as a runtime option" rows clean.
* The compared `Outcome` is `(wait status, return value, cp_error_reason string,
  the whole scratch region, the normalised assert diagnostic)`. The assert
  diagnostic is captured from the child's stderr through a pipe and normalised
  to ``lib.c:{line}: {func}: Assertion `{expr}' failed.``, so "the *same* assert
  fired" is a checked property.
* `run()` is serialised on the mutex that owns the single scratch region.
* A child that exits *normally* without filling in the result header raises a
  loud `HARNESS ERROR` instead of being reported as a library outcome.

## Test inventory

| file | tests | what it covers |
|------|-------|----------------|
| `tests/symbols_diff.rs` | 6 | `nm -D` parity (names **and** sizes), `RTLD_NOW` resolution, `static` helpers absent from both, default contents of all six writable tables, `cp_state_t` layout vs. a C probe |
| `tests/unfilter_diff.rs` | 19 | `CONFIGS.md` rows 1..22 for `unfilter`: `h <= 0`, all five filters on row 0 and on later rows, the full 5×5 filter cross product, per-row random filters over 3..12 rows, `bpp ∈ {0,1,2,3,4,8,16,33}`, `bpp == len`, `bpp > len`, `w == 0`, negative `w`/`bpp`/`len`, pointer skew, a 1..64 × 1..48 shape sweep, `cp_paeth`/Average value patterns, and an exhaustive `w ∈ -2..5 × h ∈ -1..3 × bpp ∈ -2..4 ×` all filter combinations |
| `tests/unfilter_errors.rs` | 8 | `ERRORS.md` rows 9..13: all 251 invalid row-0 filter bytes, invalid filters on later rows (with the partial mutation), `h <= 0` touching nothing, `raw == NULL`, the `raw[x] += 0` prologue walking off the buffer, `INT_MIN`/`INT_MAX` scalars |
| `tests/inflate_valid.rs` | 24 | `CONFIGS.md` rows 23..64: every literal value, every length code (0..28) and distance code (0..29) with boundary and random extra bits, `distance == 1` (memset path), non-overlapping and overlapping copies, dynamic blocks over the full `HLIT`/`HDIST`/`HCLEN` ranges and all four code-length run modes, 9..14-bit trees, stored blocks 0..2051 bytes at all four alignments, stored-after-bit-packed, 2..4-block streams with cross-block back-references, all `in` alignments and `in_bytes` residues, exact/oversized `out_bytes`, third-party (`flate2`) streams at levels 0/1/6/9, and each of the six writable tables overridden |
| `tests/inflate_errors.rs` | 23 | `ERRORS.md` rows 1..8 and 14..36: all six error strings including their check *order*, `out_bytes` 0/negative, `out == NULL`, `in == NULL`, `in_bytes` 0/negative/`INT_MIN`/`INT_MIN+1`, the unchecked stored-block over-read, and all six reachable `assert()` sites (two of them via hand-derived inputs) plus a proof sketch for the four unreachable ones |
| `tests/inflate_fuzz.rs` | 6 | 5881 randomized corrupt `cp_inflate` inputs (random bytes, long random bytes, truncations, single-bit mutations, random dynamic headers) and 4000 randomized `unfilter` argument sets, every divergence checked against the UB oracle |

Total: **86 tests**, all passing in every feature combination × cargo profile ×
cdylib profile that `run_all.sh` enumerates.

## Changes made to the translation

| # | change | why |
|---|--------|-----|
| 1 | `c-asserts` cargo feature (**on by default**) that translates all 10 `assert()`s from `c_src/src/lib.c`, writing a glibc-shaped diagnostic to stderr and calling `abort()` | the reference `.so` is built with **no** `CMAKE_BUILD_TYPE`, so `NDEBUG` is *not* defined and `__assert_fail` is linked in; the previous translation dropped the asserts, so it returned a value where the reference library died with `SIGABRT`. `--no-default-features` reproduces a `-DNDEBUG` build instead |
| 2 | `cp_decode`'s assert uses `wrapping_shr` | `len = 32 - (key & 0xF)` is 32 when the tree entry has a zero code length, and gcc emits a 32-bit variable shift (`shr %cl, %esi`), i.e. the count is taken mod 32 |
| 3 | `unfilter`'s `for (x = 0; x < bpp; x++) raw[x] += 0;` prologue is now performed with `read_volatile`/`write_volatile` instead of being folded away | the add is a no-op but the access is not: with a large `bpp` the C library faults there, and LLVM deletes a non-volatile `x += 0` |
| 4 | `[profile.dev] overflow-checks = false, debug-assertions = false` | the crate reproduces the C source's UB (writing through a NULL `out`, negative-stride `unfilter` accesses, wrap-around arithmetic); Rust's debug UB checks turned those into Rust aborts, so a `cargo build` (dev) artifact behaved differently from the C library. With this, the dev-profile cdylib passes the same suite as the release one |
| 5 | `[features]`, `[dev-dependencies]` (`libloading`, `libc`, `flate2`) | test harness |

No other behavioural change was needed: everything else in
`translation/src/lib.rs` already matched, including the quirks listed below.

## C quirks that the translation reproduces (verified, not "fixed")

* `cp_stored`'s length check is **inverted**: it rejects a stored block when
  *more* input remains than `LEN` announces, so a stored block is only accepted
  as the last thing in the stream — and then `memcpy(s->out, p, LEN)` runs with
  **no** output-bounds check at all.
* `cp_peak_bits` adds `s->bits_left` (not `last_bytes * 8`) to `count` when it
  loads the final partial word, permanently over-counting. That is what makes
  `cp_ptr`'s source pointer wrong for stored blocks, and it is also the only way
  to reach `cp_ptr`'s and `cp_read_bits`'s asserts.
* `cp_build` returns `first[15]`, which does **not** count the length-15 codes,
  so symbols with 15-bit codes are outside the range `cp_decode` searches.
* `cp_decode` reads `tree[lo - 1]`, i.e. one `u32` *before* `lit`/`dst`/`len`
  inside `cp_state_t`, whenever its binary search ends at `lo == 0`. The
  translation derives all three sub-array pointers from the base of the same
  allocation and uses a `#[repr(C)]` struct with the verified layout, so this
  reads the same bytes.
* `unfilter`'s row-0 `case 2` (Up) is a no-op, `case 4` (Paeth) degenerates to
  `case 1` (Sub), and the row-0 `case 1/3/4` loops start at `x = bpp` while the
  later rows' `case 1` starts at `x = 0`.
* `unfilter` validates nothing: `NULL`, negative `w`/`h`/`bpp`, and
  `w * bpp` overflow all just happen.

## Known, documented divergences

Exactly one class, and it is provably undefined behaviour in the C source:
`cp_dynamic` writes past `uint8_t lens[288 + 32]`, and in the reference build
gcc puts that function's own `int` locals right behind that array — so at
`n == 364` the code-length loop zeroes its own counter and the C library spins
forever. See `ERRORS.md` rows 35/36 and the "unavoidable divergences" section.

The harness proves the classification mechanically rather than asserting it: it
builds the same unmodified `c_src/src/lib.c` a *second* time with
`-fstack-protector-all --param=ssp-buffer-size=1`, which only moves the
function-local variables. An input whose behaviour differs between those two C
builds cannot depend on anything but the frame layout. Measured over 5881
randomized corrupt inputs: 37 layout-dependent, **0 unexplained**.
