# Verification notes

## What is being verified

`c_src/src/main.c` (45 lines, three functions) against `src/logic.rs` +
`src/lib.rs` + `src/main.rs`.  Two artefacts are compared, always through the
dynamic-symbol boundary:

| artefact | C | Rust |
|----------|---|------|
| shared object | `build_c/libcdriver.so` (`gcc -shared -fPIC -fno-strict-aliasing`) | `target/<profile>/libdriver.so` (`crate-type = ["cdylib"]`) |
| executable | `c_src/build/driver` (CMake) | `target/<profile>/driver` |

Exports (`nm -D --defined-only`): `driver`, `main` — identical in both `.so`s
(`print_hex` is `static` in C and is therefore exported by neither).

## How to run everything

```
./run_all.sh              # C build + every feature combo + every profile + tests + symbol parity
./check_features.sh       # cargo check for the full feature power-set (the crate declares none)
./symbol_parity.sh debug  # nm -D diff between the two .so files
cargo test                # the differential suite alone (builds the C .so on demand)
```

Test layout (`tests/`):

| file | phase | what it drives |
|------|-------|----------------|
| `common/mod.rs` | harness | dlopen both `.so`s, fork-based fd 0/1/2 plumbing, SplitMix64 RNG |
| `phase_b_driver.rs` | B | `driver` export, rows 1-9 + 30 of CONFIGS.md |
| `phase_b_main.rs` | B | `main` export, rows 10-26 + 29 |
| `phase_b_stdio_state.rs` | B | rows 31-35 (bytes consumed from fd 0, repeated `main`, leftovers) |
| `phase_b_exe.rs` | B | rows 27-28, both real programs end to end |
| `phase_c_errors.rs` | C | one test per ERRORS.md row |
| `phase_d_symbols.rs` | D | `nm -D` parity, dlsym-ability, no extra/unresolved symbols |
| `fuzz_soak.rs` | B/C soak | `#[ignore]`d, ~30 s: 200 000 random `i32` through `driver`, 20 000 random stdin blobs, 4 000 descriptor-state cases, 4 000 end-to-end runs, digit lengths 1..400 (`cargo test -- --ignored`) |

Every call is made in a `fork`ed child that sets up fd 0/1/2 itself, so
(a) libtest's own progress output can never leak into the captured bytes,
(b) no stdio state survives between invocations unless a test asks for it
(`call_main_n`), and (c) tests can still run in parallel.

## Behaviours that were deliberately reproduced (not "fixed")

1. **`scanf`'s return value is ignored.**  On EOF, read error, or matching
   failure, `x` keeps its initial `0`, and the program still exits `0`.
2. **glibc's `%d` conversion is `strtol` + truncation.**  Out-of-`long` inputs
   saturate to `LONG_MAX`/`LONG_MIN` and are then truncated to `int`, so
   `"99999999999999999999"` prints `ffffffff` and its negation prints
   `00000000`.  Inputs that fit `long` but not `int` are silently truncated.
3. **The terminating byte is pushed back, not consumed** (`"42 rest"` consumes
   only `42`).
4. **stdin is read exactly the way glibc's `FILE` reads it**, because that is
   observable on the descriptor:
   * one process-wide buffer (so two `main()` calls in one process continue where
     the previous conversion stopped, and EOF is sticky while a read error is
     not);
   * buffer size = `st_blksize` when `0 < st_blksize < BUFSIZ`, else `BUFSIZ`
     (8192) — glibc's `_IO_file_doallocate`.  On this filesystem that is 4096,
     which is why a 20 002-byte pipe leaves exactly 15 906 bytes unread in both
     implementations;
   * one `read(2)` per refill, no retry on `EINTR` (glibc sets `_IO_ERR_SEEN`
     and reports EOF for that attempt instead of restarting the syscall).
5. **libc's exit-time `_IO_cleanup`.**  When the *program* exits, glibc seeks
   stdin back over the bytes it buffered but never consumed (`_IO_new_file_sync`,
   `ESPIPE` ignored).  `src/main.rs` calls `program_main(true)` to reproduce that
   side effect; the `main` exported from the cdylib uses `program_main(false)`
   because returning from a `dlsym`'d `main` does not run libc's cleanup — which
   is exactly what the C `.so` does too.  Verified in
   `cfg_31`/`cfg_32` (`.so`, no cleanup) and `cfg_34`/`cfg_35` (program, cleanup).

## Known, unavoidable limits of a non-libc translation

These only exist for the *shared-library* use of a translation unit that was
written as a program (`c_src/CMakeLists.txt` builds an executable), and they are
properties of libc's global `FILE` objects, not of the translated logic:

* **The C `.so` shares the host process' `stdin`/`stdout` `FILE`s; the Rust `.so`
  cannot.**  If the host has already driven glibc's `stdin` to EOF (e.g.
  `python3 - <<EOF` feeds the script on fd 0), the C `main` returns immediately
  because EOF is sticky on *that* `FILE`, while the Rust `main` reads fd 0.  With
  a pristine stdio state (any normal caller, and every test here) the two agree
  byte for byte — independently double-checked with a Python/`ctypes` harness
  that dlopens both objects and calls `main()` four times:
  `['01000000','02000000','03000000','00000000']` from both.
* **stdout buffering.**  C's `printf` leaves the line in glibc's buffer until the
  process exits (or the caller flushes); the Rust `driver` writes it to fd 1 and
  flushes immediately.  The *bytes* are identical, only the moment they leave the
  process differs, so a differential harness must flush the C side before reading
  the capture — `fflush(NULL)` in `fork_capture`, and glibc's own exit-time flush
  for the executables.  For the same reason `call_main_and_drain` reports the
  leftover stdin bytes on fd 2 *before* flushing: on glibc, `fflush(NULL)` also
  syncs *input* streams and would seek the descriptor back, i.e. it would measure
  the harness rather than the implementation.
* **A tty on fd 0** cannot be exercised in this sandbox (no controlling
  terminal).  glibc's line-buffering flag only affects *output* streams, and the
  input path is the same block-read code, so no divergence is expected.

## Environment

* `rustc`/`cargo` 1.94.0, `gcc` (x86-64 Linux, glibc), `cmake` 3.x
* little-endian, `sizeof(int) == 4`, `sizeof(long) == 8`, `st_blksize == 4096`
* offline: only `libloading` 0.8 and `libc` 0.2 as dev-dependencies (both already
  vendored in the local cargo registry); the randomized inputs use a
  self-contained SplitMix64 generator with a fixed seed, so runs are
  reproducible.
