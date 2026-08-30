# Rust translation of `c_src` (cute_png-derived PNG/DEFLATE decoder)

`src/lib.rs` is a literal, ABI-compatible translation of `c_src/src/lib.c`.
It is built as a `cdylib` and exports exactly the same public symbols as the
shared library produced by `c_src/CMakeLists.txt`.

## The reference build

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference library is
compiled at **`-O0` and without `-DNDEBUG`**. Two consequences drive this
translation and are easy to get wrong:

1. **`assert()` is live.** A failing assert calls `__assert_fail`, prints to
   stderr and raises `SIGABRT`. The `.so` imports `__assert_fail`, which is how
   you can check this. Every one of the ten asserts in `c_src/src/lib.c` is
   reproduced here by `c_assert!`, which calls libc `abort()`. Malformed
   DEFLATE data therefore *aborts the process* far more often than it returns
   an error — for example `cp_inflate(in, 0, out, n)` aborts on
   `assert(s->bits_left > 0)` before it can do anything else.
2. **`-O0` fixes the stack layout**, which matters because `cp_dynamic` writes
   past the end of a local array into its own locals. See below.

## Exported ABI

`nm -D --defined-only` on both libraries yields the same 9 public symbols:

| symbol | type | size |
|---|---|---|
| `load_png_mem` | FUNC | - |
| `cp_inflate` | FUNC | - |
| `cp_error_reason` | OBJECT (`.bss`) | 8 |
| `cp_fixed_table` | OBJECT (`.data`) | 320 |
| `cp_permutation_order` | OBJECT | 19 |
| `cp_len_extra_bits` | OBJECT | 31 |
| `cp_len_base` | OBJECT | 124 |
| `cp_dist_extra_bits` | OBJECT | 32 |
| `cp_dist_base` | OBJECT | 128 |

`diff` of the sorted `nm -D --defined-only` name lists is empty (see
`SYMBOLS.md` and `run_verification.sh`). There are no namespace/renaming macros
in the C headers, so linker names equal source names. The six tables and
`cp_error_reason` are mutable globals in C and are `static mut` here; the
decoder reads them through raw pointers on every call, so a caller that mutates
them changes behaviour exactly as it does in C.

`cp_image_t` (16 bytes: `int w; int h; cp_pixel_t *pix;`) is `#[repr(C)]` and
returned by value from `load_png_mem`, matching the SysV AMD64 register return
(RAX/RDX). `cp_state_t` is `#[repr(C)]` and its layout is checked against the
C compiler by a unit test (size 2464, `lookup` @72, `lit` @1096, `dst` @2248,
`len` @2376) — this matters because `cp_decode` reads `tree[-1]` when a Huffman
tree is empty, i.e. it reads the *preceding struct field*.

Memory is allocated with libc `malloc`/`calloc` and released with libc `free`,
so pointers handed back to callers (`img.pix`) can be `free()`d by C code, and
`cp_inflate`'s alignment-sensitive logic (`first_bytes = ((in + 3) & ~3) - in`)
sees the same pointer alignments.

## Fidelity notes

Bugs and quirks of the C are reproduced deliberately, including:

* `w = cp_make32(ihdr) + 1` (the +1 in the C source) and `img.w = w - 1`.
* `cp_stored`'s `s->bits_left / 8 <= LEN` check, which rejects any stored
  DEFLATE block that is not the last thing in the stream, and its `memcpy` that
  ignores `out_end`.
* `cp_unfilter`'s first row: filter 2 is a no-op, filter 4 uses
  `cp_paeth(raw[x-bpp], 0, 0)`, filter 1/3/4 start at `x = bpp`.
* `cp_decode`'s `tree[lo - 1]` read when `hi == 0`, and its
  `assert((search >> len) == (key >> len))` where `len` is 32 whenever
  `key & 0xF == 0` — gcc emits a 32-bit `shr %cl`, whose count is taken mod 32,
  so the shift is a no-op and the assert degenerates to `search == key`.
* Signed vs unsigned pointer arithmetic: `cp_chunk` uses `int offset = len + 12`
  (sign-extended, so a chunk length >= 0x80000000 walks *backwards*), while
  `cp_find` uses `png->p += len + 12` (uint32, zero-extended).
* All integer truncation/wrap-around (`datalen += len` through `uint32`,
  `pix_bytes` truncated to `int`, `cp_make32` shifting into the sign bit, ...).
* The exact order of every validation check and the exact error strings,
  including the misspelled `"innapropriate window size detected"`.
* `cp_error_reason` is *not* cleared on success, so a stale reason survives.

### The `.data` layout, and the C's out-of-range table reads

`cp_block` indexes `cp_len_extra_bits`/`cp_len_base` with `symbol - 257` and the
`cp_dist_*` tables with a distance symbol. When a corrupt Huffman tree makes
`cp_decode` return an out-of-range symbol (up to 4095 — e.g. `tree[-1]` on the
distance tree yields 287), the C reads *past* the end of a table into whatever
the linker placed next. In the reference library those six tables are the whole
of `.data` — 0x2a0 = 672 bytes at 0x6060 — laid out in **source order**, each
table 32-byte aligned, gap bytes zero (`objdump -s -j .data`):

```
rel   0  cp_fixed_table        320 B
rel 320  cp_permutation_order   19 B  + 13 pad
rel 352  cp_len_extra_bits      31 B  +  1 pad
rel 384  cp_len_base           124 B  +  4 pad
rel 512  cp_dist_extra_bits     32 B
rel 544  cp_dist_base          128 B    -> ends at rel 672 == .bss
```

`.bss` follows immediately: `completed.0` (8 B, rel 672..680) then
`cp_error_reason` (8 B, rel 680..688). The RW `LOAD` segment ends at 0x6310 and
the mapping is page-rounded to 0x7000, so rel 688..4000 reads as zero and
rel >= 4000 faults.

Rust/LLVM order and align statics differently and that order cannot be
controlled portably, so the four indexed reads resolve their index through a
model of the layout above (`blob_byte`) instead of walking off the end of a Rust
static. The model reads the *live* statics, so a caller that mutates a table
still affects out-of-range reads just as in C. `cargo test` checks the model
against the reference layout byte for byte, including `cp_len_extra_bits[32] == 3`
(the LSB of `cp_len_base[0]`) and `cp_dist_extra_bits[32] == 1` (the LSB of
`cp_dist_base[0]`).

One case remains outside anyone's control: if the *consumer* declares the tables
`extern` and links against the library, the linker emits copy relocations, the
tables move into the consumer's `.bss`, and the C's out-of-range reads then
depend on the consumer's layout. Consumers that only call the two functions, or
use `dlopen`/`dlsym`, are unaffected.

### `cp_dynamic`'s stack frame

`cp_dynamic` writes past the end of its `uint8_t lens[288 + 32]`: the 16/17/18
run-length code-length symbols are bounded only by their repeat count, so `n`
can run up to 137 entries beyond `nlit + ndst`. At `-O0` gcc lays the frame out
like this (`sub rsp, 0x190`, offsets from `rbp`, read off `objdump -d`):

```
rbp-0x188  s (spilled parameter)      rbp-0x180  lens[320]
rbp-0x40   lenlens[19] (+5 pad)
rbp-0x24   sym    rbp-0x20 nlen   rbp-0x1c ndst   rbp-0x18 nlit
rbp-0x14   i(18)  rbp-0x10 i(17)  rbp-0xc  i(16)  rbp-0x8  n   rbp-0x4 i(perm)
```

so `lens[k]` for `k >= 320` aliases `lenlens` (320..339), padding (339..348),
`sym`, `nlen`, `ndst`, `nlit`, the three loop counters, `n` itself (376..380) and
the permutation counter. Zeroing `nlit`/`ndst` changes the loop bound and the
two trailing `cp_build` calls; zeroing `n`'s low byte snaps `n` back to 256.
`src/lib.rs` models the frame byte-exactly and performs every access through it
in the same order as the `-O0` code, including the reloads-from-memory that make
the clobbering observable. `lens[-1]`, read when symbol 16 arrives at `n == 0`,
is the most significant byte of the spilled `s` pointer, i.e. 0 for any heap
pointer, so that read is well defined rather than indeterminate.

Writes at `k >= 384` would hit the saved `rbp` / return address; the analysis in
the source shows they are unreachable (`n` is always snapped back at `k == 376`
first, and the default `lens[n++] = sym` case cannot get past `k == 319` because
the loop guard caps `n < nlit + ndst <= 320`), but if one ever happened the
translation replays the C's `SIGSEGV`.

### `cp_build`'s `counts[16]` overrun

`counts[lens[n]]++` indexes a 16-entry array with a `uint8_t`, so a code length
`>= 16` writes into the adjacent `first`/`codes` arrays (and beyond). It does not
matter: the second loop's `assert(len < 16)` fires for exactly the same inputs,
so the process always dies before the corrupted values can be used. The
translation uses 256-entry arrays (memory-safe, identical values in `[0,16)`)
and aborts at the assert.

### Deviations that remain

Three places where the C's behaviour is genuinely not reproducible:

1. Blob offsets 680..688 model `cp_error_reason`, a runtime pointer value; the
   two libraries necessarily hold different addresses there. Reaching it needs
   a corrupt tree that yields exactly `cp_len_base[74]`, `cp_dist_base[34]`,
   `cp_len_extra_bits[328]` or `cp_dist_extra_bits[168]`.
2. Blob offsets >= 4000 fault in the reference mapping; the model returns 0.
3. `cp_dynamic`'s frame padding at `lens[339..348]` is uninitialised stack in
   the C (leftovers from the `cp_read_bits`/`cp_decode` frames below it); the
   model uses zero. Only observable when a symbol-16 run copies out of that
   window, i.e. after the array has already been overrun by 19+ bytes.

## Verification

`ERRORS.md` (the error surface), `CONFIGS.md` (the valid-input surface) and
`SYMBOLS.md` (symbol parity) are derived mechanically from the C source and the
built `.so`. `tests/` contains a differential harness that `dlopen`s **both**
libraries and calls them only through their exported symbols:

| file | what it covers |
|---|---|
| `tests/harness/mod.rs` | `dlopen`/`dlsym`, fork-isolated execution, outcome comparison, PRNG |
| `tests/harness/make.rs` | canonical-Huffman DEFLATE and PNG generators |
| `tests/smoke.rs` | harness self-check + one valid PNG decoded to known pixels |
| `tests/phase_b_inflate.rs` | `CONFIGS.md` rows 1-20 (`cp_inflate`) |
| `tests/phase_b_png.rs` | `CONFIGS.md` rows 21-52 (`load_png_mem`) |
| `tests/phase_c_errors.rs` | every row of `ERRORS.md` |
| `tests/fuzzcommon/mod.rs` | corpus + mutation operators shared by the fuzz targets |
| `tests/fuzz_inflate.rs` | mutated/random DEFLATE streams |
| `tests/fuzz_inflate_tables.rs` | the same with the exported tables retuned per call (reaches `ERRORS.md` A7/A8) |
| `tests/fuzz_png.rs` | mutated PNG containers |

Because a failing `assert()` kills the process, every call runs in a forked
child and a batch is replayed from the case *after* the one that died. An
outcome is therefore one of `Ret(bytes)`, `Signal(n)` or `Exit(n)`, and "both
abort with SIGABRT" is distinguished from "both return the same error string".
Buffers handed to the libraries are 16-byte aligned at a chosen `% 4` offset and
their surroundings are filled with a deterministic pattern, so the C's
deliberate out-of-bounds *reads* (past the input, past a `PLTE` chunk, before
the input via `cp_ptr`) are reproducible. Three more details matter:

* **Table mutations are undone after every case.** A child runs many cases in a
  row and the C and Rust children restart at different points (whenever one of
  them aborts), so a leaked mutation would make the two disagree for reasons
  that have nothing to do with the translation.
* **Core dumps are disabled in the child** (`RLIMIT_CORE = 0`). With the default
  `core_pattern` piping to `systemd-coredump`, each of the thousands of
  deliberate aborts costs ~150 ms.
* **Each case has a `SIGALRM` watchdog** (3 s). A retuned `cp_len_base` can give
  the C a negative `int length`, whose `while (length--)` writes gigabytes before
  faulting; both libraries get the same budget, so a timeout is just another
  comparable outcome.

Run everything:

```
./run_verification.sh          # builds C + Rust, diffs symbols, runs all feature combos
# or, per target (the error-path and fuzz targets fork thousands of children):
cargo test --offline --release --test phase_b_inflate -- --test-threads=1
cargo test --offline --release --test phase_c_errors  -- --test-threads=1
FUZZ_ROUNDS=20 cargo test --offline --release --test fuzz_inflate -- --test-threads=1
```

`FUZZ_ROUNDS` scales the fuzz volume (default 2 rounds x 250 cases for the two
`cp_inflate` fuzzers, 3 x 800 for the PNG fuzzer).

`Cargo.toml` declares no `[features]`, so there is exactly one feature
combination; `run_verification.sh` still runs `--no-default-features` to prove
it, and would enumerate the power set if features were added.

### What is deliberately *not* compared

Three classes of input make the C library nondeterministic *by itself* (two runs
of the same C `.so` disagree), so they are excluded rather than papered over:

* **Heap overflows.** `cp_stored`'s `memcpy` ignores `out_end`, and in
  `load_png_mem` `out_end` sits `(w+1)*h*(4-bpp)` bytes *past* the end of the
  `img.pix` allocation, so overrunning it corrupts the heap. Whether glibc
  notices depends on the heap layout. The `cp_inflate` fuzzer allocates 70 000
  bytes of slack so a stored block's `memcpy` lands in our own buffer, and the
  PNG tests keep the decoded size within `img.pix`.
* **Reads of uninitialised `malloc` memory.** A DEFLATE stream that produces
  *fewer* bytes than `h * (1 + w*bpp)` leaves part of the scanline block
  unwritten, and `cp_unfilter`/`cp_convert` then read it.
* **Two specific table mutations.** Setting a *high* byte of a `cp_len_base` /
  `cp_dist_base` entry makes `int length` / `int backwards_distance` negative,
  and the C's `while (length--)` then writes gigabytes past the buffer (or
  `memset` gets a huge `size_t`); setting `cp_permutation_order[i] >= 64` makes
  `cp_dynamic` write past its own `rbp` into the saved frame pointer, return
  address and its caller's frame. Values `0..=63` and low-byte-only base
  mutations *are* compared, and are modelled exactly.

`tests/fuzz.rs` handles both automatically: `fuzz_same` runs the C corpus twice
from differently-aged parent heaps and only compares the cases on which the C
agreed with itself, reporting how many were dropped.
