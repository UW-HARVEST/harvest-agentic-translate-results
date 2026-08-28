# Verification report — `driver` C → Rust translation

## How to reproduce

```bash
# build the C reference + the Rust cdylib, then run the whole suite
cd translation && ./scripts/run_tests.sh            # debug
cd translation && ./scripts/run_tests.sh --release  # release

# every feature combination declared in Cargo.toml, both profiles
cd translation && ./scripts/check_features.sh
```

`scripts/run_tests.sh` always runs `cargo build` before `cargo test`, because
`cargo test --test <name>` does **not** relink the `cdylib` that the tests
`dlopen` (the test binaries have no Rust-level dependency on it). The harness
additionally refuses to run against a `.so` older than `src/`.

Every test loads BOTH shared objects with `libloading` and calls only their
exported C symbols — the Rust `#[no_mangle] extern "C"` wrappers are therefore
part of what is under test. No Rust function is ever called directly.

## Results

| | debug | release |
|---|---|---|
| `tests/low_level.rs` (os_calloc / os_realloc / os_strdup / merror / FreeAlertData) | 13 pass | 13 pass |
| `tests/get_alert_data.rs` (GetAlertData) | 32 pass | 32 pass |
| `tests/file_queue.rs` (Init_FileQueue / Read_FileMon) | 24 pass | 24 pass |
| `tests/driver.rs` (driver) | 11 pass | 11 pass |
| **total** | **80 pass, 0 fail** | **80 pass, 0 fail** |

`scripts/check_features.sh` output: `ALL FEATURE COMBINATIONS PASSED`
(default and `--no-default-features`, × debug and release — `Cargo.toml`
declares no `[features]`, so those are all the configurations that exist).

## Completion gate

- [x] **`SYMBOLS.md`** — `diff` of `nm -D --defined-only` between the C `.so`
      and the Rust `.so` is **empty** for both the debug and the release
      cdylib. All 9 C symbols (`os_calloc`, `os_realloc`, `os_strdup`,
      `merror`, `FreeAlertData`, `GetAlertData`, `Init_FileQueue`,
      `Read_FileMon`, `driver`) are exported with the exact same names.
      `nm -D -u` on the Rust `.so` shows no undefined non-libc/non-libgcc
      symbol. No stubs, no `unimplemented!()` — every C translation unit has a
      complete Rust counterpart.
- [x] **Phase B** — all 42 rows of `CONFIGS.md` pass, each driven with many
      seeded-random inputs (splitmix64, fixed seeds), including the low-level
      entry points (`GetAlertData` on caller-owned `FILE*`, `Read_FileMon` on a
      hand-built `file_queue`) and not just the `driver()` convenience wrapper.
- [x] **Phase C** — all 34 rows of `ERRORS.md` have a passing differential
      test asserting the *same* sentinel / error code / signal / stderr text,
      plus the generic boundaries (NULL pointers, zero and oversized lengths,
      values one past the documented ranges, and out-of-range "enum" bit
      patterns for `flags` — `i32::MIN`, `i32::MAX`, `-1`, random words).
- [x] **Feature combinations** — verified for every combination in both
      profiles.

## Bug found and fixed in the Rust translation

**`src/file_queue.rs:218` and `:275` — `fileq->year = p->tm_year + 1900`.**

The translation used a plain `+`, so an overflow-checked Rust build (any debug
build, and the default `cargo build` profile) **panicked** — and because the
function is `extern "C"` the panic became a non-unwinding abort, i.e. the whole
process died with `SIGABRT`. The C performs ordinary `int` addition, which just
wraps. Reproduced by `tests/driver.rs::cfg38_driver_extreme_day_year` with
`year = i32::MAX`. Fixed to `wrapping_add(1900)`; the fix makes debug and
release builds agree with the C for every `int` input.

## Additional fidelity tweaks (no observable change, but they remove the last
## theoretical divergences an independent statement-by-statement audit found)

* `src/file_queue.rs` — `merror(FSEEK_ERROR/FSTAT_ERROR, file_name, errno, strerror(errno))`.
  Disassembly of the C `.so` shows gcc evaluates `strerror(errno)` **first** and
  re-reads `errno` for the `%d` argument afterwards, whereas Rust evaluates
  arguments left to right. The message is now bound to a local before the
  `errno()` read, so both libraries read `errno` in the same order. (Unobservable
  with glibc, which does not clobber `errno` in `strerror` for a valid code — and
  the only codes reachable here are `ESPIPE`/`EBADF`.)
* `src/read_alert.rs` — `z = strlen(p) - strlen(m)` now uses `wrapping_sub`.
  `m = strstr(p, ":")` always points inside `p`, so the subtraction cannot
  actually underflow, but the C is `size_t` arithmetic that wraps, and this
  removes any possibility of an overflow-check panic inside an `extern "C"` fn.
* `src/cbind.rs` — glibc declares `struct stat::__pad0` as `int`, so the field
  type was changed from `c_uint` to `c_int` (same size/offset; never read).

The struct layouts were also checked against a C probe compiled with the real
headers, and the expected `sizeof`/`offsetof` values are now asserted in
`tests/low_level.rs::cfg05b_struct_layout_matches_c`:
`alert_data` 96 bytes (0,4,8,16,24,32,40,48,56,64,72,80,88),
`file_queue` 440 bytes (0,8,12,16,20,24,288,296),
`struct stat` 144 bytes (`st_size`@48, `st_mtim`@88),
`struct tm` 56 bytes (`tm_mon`@16, `tm_year`@20).

## Documented, deliberate divergence (UB input only)

`file-queue.c:125` / `:166` do `strncpy(fileq->mon, s_month[p->tm_mon], 3)`
with **no bounds check** on `tm_mon`, while `s_month` has exactly 12 entries.
For `tm_mon` outside `0..=11` the C reads past the table — undefined
behaviour, and on this build a hard crash: `s_month` occupies all of `.data`
(`0x5140..0x51a0`, 12 × 8 bytes) and `.bss` follows, so `s_month[12]` is
`NULL` and `strncpy(dst, NULL, 3)` raises `SIGSEGV`. Negative indices read
`.got.plt`.

`src/file_queue.rs::copy_month` range-guards the copy instead, so the Rust
survives. This is recorded as `ERRORS.md` row 27 and proved by
`tests/file_queue.rs::err27_out_of_range_month_is_ub_in_c`, which asserts that
the C really does die (there is no defined behaviour to match) and that all
twelve in-range months still match byte-for-byte. Note that `fileq->mon` is
never read anywhere in the library and is not reachable through `driver()`, so
the divergence is unobservable for the public one-shot API.

## Notes on inherently non-deterministic comparisons

* `GetAlertData`'s `char str[OS_MAXSTR + 1]` is uninitialised in the C (only
  `str[OS_MAXSTR]` is set) and zero-initialised in the Rust. The only input
  that can read the uninitialised part is a final line consisting of exactly
  `** Alert` with no `'\n'` — and because `fgets` stops at newlines such a line
  must be the *last* one, so `_r` can never reach 2 and the function returns
  `NULL` with the stream at EOF regardless of what the garbage contains. The
  case is covered by `cfg14_no_trailing_newline`.
* `QueueSnap` compares everything in `file_queue` (including the whole 257-byte
  `file_name` buffer, `flags`, `mon`, `last_change` and the `struct stat`
  identity/size/mtime fields) except `st_atim` and `st_ctim`, which change
  merely by opening or re-permissioning the queue file between the C run and
  the Rust run. `last_change`, which is the only `f_status` field the library
  ever consumes, *is* compared.
* Rust debug builds turn a null-pointer dereference into a panic/`SIGABRT`
  while the C (and the release Rust build) raise `SIGSEGV`. The
  `__attribute__((nonnull))`-violating inputs are therefore asserted to be
  fatal in both builds and to produce the *identical* signal for the release
  library (`tests/get_alert_data.rs::err17_null_pointer_args_both_segv`).
* `Read_FileMon` sleeps `FQ_TIMEOUT` = 5 s per miss (`select` with a timeout),
  so the tests use `timeout = 0` wherever possible; the four tests that must
  exercise the sleeping paths assert the elapsed time as well.
