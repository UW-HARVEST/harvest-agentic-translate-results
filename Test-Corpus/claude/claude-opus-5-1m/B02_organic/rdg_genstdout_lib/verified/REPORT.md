# Verification report

Library under test: `c_src/src/lib.c` (2 exported functions, extracted from
zstd's `programs/fileio.c`) vs. its Rust translation in `src/lib.rs`.

Reproduce everything with:

```
./run_all.sh
```

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md` — `nm -D` diff C→Rust | **empty** (2/2 symbols, debug *and* release profile); 0 missing / undefined non-libc symbols |
| Phase B — every `CONFIGS.md` row (25) passing over randomized inputs | **pass** |
| Phase C — every `ERRORS.md` row (14, + 2 extra boundary tests) passing | **pass** |
| Every feature combination | **pass** — `Cargo.toml` has no `[features]` table, so there is exactly one valid combination; `--no-default-features`, default and `--all-features` are all run, in the dev and release profile |

44 differential tests total, all green in both profiles:

```
tests/valid_paths.rs   25 passed   (CONFIGS.md rows 1-25)
tests/error_paths.rs   16 passed   (ERRORS.md rows 1-14 + 2 extra)
tests/symbols.rs        3 passed   (nm -D parity + dlsym resolvability)
tests/child.rs          harness=false helper executable
```

Both libraries are always loaded with `libloading` and driven only through their
exported C symbols — the Rust functions are never called directly, so the
`#[no_mangle] extern "C"` wrappers are themselves under test.

## Build-time configuration surface

* `Cargo.toml`: no `[features]`, no optional dependencies ⇒ the complete set of
  valid feature combinations is the single empty combination. Enumerated
  mechanically by `run_all.sh` (it computes the power set of whatever the
  `[features]` table declares, so it stays correct if features are added).
* `c_src/CMakeLists.txt`: a single `SHARED` target, no options, no
  `add_definitions`. The only conditional compilation is
  `#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MSVCRT__)` inside
  `lib.c`, which selects `'\\'` plus a second `extractFilename` pass. On this
  host the `#else` branch is compiled; `src/lib.rs` mirrors that with
  `#[cfg(windows)] / #[cfg(not(windows))]`, so the compiled configurations match.
* Both the dev and the release profile are verified. This matters here: the
  release profile disables rustc's debug-only UB checks, so the two profiles are
  genuinely different code paths for a library that deliberately performs
  out-of-bounds reads.

## Divergence found and fixed

**Rust aborted (SIGABRT) where C segfaults (SIGSEGV) on NULL input.**

The original translation reimplemented `strlen` / `strrchr` / `memcpy` as Rust
loops over raw pointers. With debug assertions enabled, rustc injects a
null-pointer-dereference check into those dereferences, so
`extractFilename(NULL, '/')` and `FIO_createFilename_fromOutDir(NULL, …)` /
`(…, NULL, …)` produced

```
thread '<unnamed>' panicked at src/lib.rs: null pointer dereference occurred
thread caused non-unwinding panic. aborting.        -> SIGABRT (signal 6)
```

whereas the C library faults inside glibc `strrchr` / `strlen` and dies with
SIGSEGV (signal 11). Caught by `err_06`/`err_07`/`err_08`.

Fix: bind the *same* libc primitives the C translation unit calls
(`strlen`, `strrchr`, `memcpy`, in addition to the already-bound `calloc`,
`fprintf`, `strerror`, `exit`, `stderr`, `__errno_location`) instead of
reimplementing them. This makes the failure modes, the `(char)c` conversion
`strrchr` performs and the zero-length `memcpy` edge cases identical by
construction, and it keeps the result `free()`-able by the caller.

A second, test-infrastructure issue was found along the way: `cargo test` does
**not** relink a `crate-type = ["cdylib"]` artifact, so the suite was initially
run against a stale `libdriver.so`. `tests/common/mod.rs::assert_fresh` now
refuses to run if `target/*/libdriver.so` is older than `src/lib.rs`, and
`run_all.sh` always runs `cargo build` before `cargo test`.

## Behaviours of the C that are reproduced on purpose (not "fixed")

1. `extractFilename(path, '\0')` — `strrchr` treats the terminating NUL as part
   of the string, so the NUL *is* found and the function returns
   `path + strlen(path) + 1`, a one-past-the-end pointer. (`ERRORS.md` rows 3-4.)
2. `FIO_createFilename_fromOutDir(path, "", n)` — `strlen(outDirName)-1` wraps to
   `SIZE_MAX`, so line 45 reads the byte *before* the buffer. The Rust computes
   the same address with `wrapping_add` (a defined pointer computation) and reads
   it. (`ERRORS.md` row 9, `CONFIGS.md` rows 16-17, verified with the preceding
   byte pinned to `'/'` and to a non-`'/'` value.)
3. The allocation size is computed with wrapping `size_t` arithmetic, so a huge
   `suffixLen` can produce an *undersized* buffer instead of a failure.
   (`ERRORS.md` rows 11-12.)
4. Allocation failure prints `zstd: FIO_createFilename_fromOutDir: %s` with
   `strerror(errno)` — no trailing newline — and calls `exit(30)`. Verified
   byte-for-byte on stderr plus the exit code. (`ERRORS.md` rows 2, 13.)

## Anti-vacuity check (mutation testing)

The suite was re-run against six deliberate mutations of `src/lib.rs`:

| mutation | detected by |
|----------|-------------|
| guard the empty-`outDirName` OOB read with `outDirLen > 0` ("fix" the C bug) | `err_09` |
| `wrapping_add(suffixLen)` → `saturating_add` | `err_11` |
| return `path` when `separator == '\0'` ("fix" the NUL case) | `err_03`, `err_04`, `err_05`, `err_15` |
| invert the trailing-separator branch | `err_09`-`err_16` (6 tests) |
| drop the `+1` so the filename overwrites the inserted separator | `err_09`-`err_16` (5 tests) |
| `strrchr(path, separator as c_int)` → `(separator as u8) as c_int` | *not* detected — and correctly so: `strrchr` converts its argument with `(char)c`, so both forms select the identical byte |
| `calloc(1, size)` → `calloc(size, 1)` | *not* detected — and correctly so: identical total size |

The four real behaviour changes are all caught; the two behaviourally equivalent
rewrites are correctly not flagged.
