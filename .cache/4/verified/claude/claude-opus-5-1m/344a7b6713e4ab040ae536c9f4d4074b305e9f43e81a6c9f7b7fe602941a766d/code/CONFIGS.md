# CONFIGS.md — configuration-surface table (Phase A → Phase B)

## Mechanical derivation of the axes

Everything below is derived from `c_src/src/main.c` and `c_src/CMakeLists.txt`.

* **Build-time configuration.** `CMakeLists.txt` is
  `cmake_minimum_required(3.10)` + `project(driver)` + `add_executable(driver
  src/main.c)` — no `option()`, no `target_compile_definitions`, no
  `CMAKE_BUILD_TYPE` branches. The C source contains **zero** `#ifdef`/`#if`
  (`grep -c '#if\|#ifdef\|#ifndef' c_src/src/main.c` → 0). `Cargo.toml` has
  **no `[features]` section**, so the only feature combination is the empty one
  (verified with `cargo check --no-default-features` and plain `cargo check`).
  The two remaining build configurations that *can* change behaviour on the
  Rust side are the cargo profiles, because `[profile.release]` sets
  `panic = "abort"` and enables optimisation; both are covered
  (`opt-level=0`/`debug-assertions=on` and `opt-level=3`/`panic=abort`).
* **Runtime options/flags.** None. `main` takes no `argc`/`argv`, reads no
  environment variable, has no locale call, no getopt, no config file. The only
  runtime inputs are (a) the `int` argument of `driver` and (b) the byte stream
  on stdin consumed by `scanf("%d", &x)`.
* **Public entry points** (`nm -D` on the C `.so`, see `SYMBOLS.md`):
  `driver` (the low-level entry point — call it directly, do not only drive it
  through `main`) and `main` (the one-shot wrapper: `scanf` + `driver`).
  `print_hex` is `static`, exercised through `driver`.
* **Input shapes the code actually distinguishes.**
  * `driver`/`print_hex`: the 4 bytes of the `int` in host order; `%02x`
    formatting distinguishes bytes `< 0x10` (zero-padded nibble) from
    `>= 0x10`, and `0x00`/`0xff` are the loop's extremes. Byte order matters
    (`&x` reinterpreted as `unsigned char*`).
  * `main`: presence/kind/length of leading whitespace, sign (`none`/`+`/`-`),
    digit count (1 … thousands), leading zeros, value magnitude relative to the
    `int`/`unsigned int`/`long` boundaries, trailing junk, trailing newline or
    not, stdin being a regular file vs a pipe, and input longer than the stream
    buffer (glibc sizes it from `st_blksize` of fd 0, i.e. 4 KiB here, capped by
    `BUFSIZ` = 8 KiB; the translation derives it the same way, so both consume
    the same number of bytes per `read`).
* **Stream state.** `driver` is stateless apart from the stdout buffer, but the
  conversion in `main` is not — and the state lives in the *stream*, not in the
  call:
  * `scanf` hands the byte it stopped on back to `stdin` (`ungetc`), so calling
    the exported `main` again continues exactly there (row C28: `"12x34"` ⇒ the
    second conversion sees `x`, `"12-34"` ⇒ it sees `-34`);
  * `scanf` reads *ahead* into the stream buffer, and at process exit glibc's
    `_IO_cleanup` seeks a seekable fd 0 back over the unconsumed part, so the
    next reader of the descriptor sees the same bytes (row C30); the buffer size
    itself comes from `st_blksize` of fd 0, so the descriptor's *type* is an axis
    too (row C37);
  * the end-of-file indicator is **sticky** — after EOF no later conversion even
    issues a `read`, however the descriptor changes underneath (row C34);
  * `scanf` holds the stream lock for the whole conversion and that lock is
    recursive, so concurrent callers cannot split a number (row C35) while a
    signal handler can still re-enter the conversion (row C36);
  * the granularity of the stdout lock (one lock per `printf`) decides how
    concurrent callers may interleave (row C31).
  The translation therefore models the stream itself (`C_STDIN`, the `atexit`
  seek-back and `CStdout` in `src/driver_impl.rs`) instead of using
  `io::stdin()`/`io::stdout()`'s private buffering.
* **Process environment.** The C code never calls `setlocale`, so `LC_*`/`LANG`
  must not change anything (row C29); the `SIGPIPE` disposition inherited from
  the process start-up is part of this axis too (see ERRORS.md E13).

The rows below are the pruned cross-product of those axes: every combination
the C code treats differently. Every row is compared byte-for-byte between the C
`.so` and the Rust `.so` loaded through `libloading` (plus, for `main`, the two
real executables). Rows whose axis is a *value* are driven by many randomised
inputs (SplitMix64 with a fixed seed, see `tests/common/mod.rs`); rows whose axis
is a *shape* are exhaustive over that shape instead (C3 covers all 8⁴ byte
combinations, C6 all 4×256 byte positions, C7 all 65 536 low halves, C2 all 2049
small magnitudes, `e4b_all_byte_values_as_prefix` all 256 leading bytes) — those
rows deliberately contain no RNG.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` (`.so`) | `x == 0` — all four bytes zero, zero-padded nibbles | `c1_driver_zero` | [x] |
| C2 | `driver` (`.so`) | small magnitudes: every `x` in `-1024..=1024` (single-byte and carry-into-second-byte shapes, positive **and** negative) | `c2_driver_small_magnitudes` | [x] |
| C3 | `driver` (`.so`) | nibble-padding shapes: every `x` whose bytes are drawn from `{0x00,0x01,0x0f,0x10,0x7f,0x80,0xf0,0xff}` (8⁴ = 4096 combinations, exhaustive) | `c3_driver_nibble_padding_matrix` | [x] |
| C4 | `driver` (`.so`) | signed boundaries: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`, `0x80000000`, `0xffffffff`, all powers of two ±1 | `c4_driver_boundary_values` | [x] |
| C5 | `driver` (`.so`) | full 32-bit range, randomised: 20 000 uniformly random `i32` (fixed seed) | `c5_driver_random_full_range` | [x] |
| C6 | `driver` (`.so`) | byte-position sweep: each of the 4 bytes takes every value `0x00..=0xff` while the others hold a fixed non-zero pattern (1024 values) — catches byte-order and per-byte formatting bugs | `c6_driver_byte_position_sweep` | [x] |
| C7 | `driver` (`.so`) | exhaustive low 16 bits: `x` covers all 65 536 values of `(fixed_high << 16) | low` | `c7_driver_exhaustive_low_16_bits` | [x] |
| C8 | `driver` (`.so`) | many calls in one process (10 000 consecutive calls, one capture) — accumulated stdio buffering, no separators | `c8_driver_repeated_calls_one_process` | [x] |
| C9 | `driver` (`.so`) | stdout is a **pipe** instead of a regular file — an unseekable, capacity-limited sink (glibc fully buffers both; the line-buffered mode needs a terminal, row C33), 5 004 randomised values | `c9_driver_stdout_is_a_pipe` | [x] |
| C10 | `main` (`.so`, forked) | no sign, plain decimal digits, randomised `0..=INT_MAX` (400 values) | `c10_main_unsigned_decimal_random` | [x] |
| C11 | `main` (`.so`, forked) | `-` sign, randomised magnitudes `0..=2^31` (400 values, includes `-0` and `INT_MIN`) | `c11_main_negative_random` | [x] |
| C12 | `main` (`.so`, forked) | `+` sign, randomised magnitudes (400 values) | `c12_main_explicit_plus_random` | [x] |
| C13 | `main` (`.so`, forked) | randomised leading-whitespace prefix built from `{' ','\t','\n','\v','\f','\r'}` (1–20 bytes) before a randomised number | `c13_main_leading_whitespace_kinds` | [x] |
| C14 | `main` (`.so`, forked) | randomised number of leading zeros (1–40) before a randomised value, with and without sign | `c14_main_leading_zeros` | [x] |
| C15 | `main` (`.so`, forked) | valid number followed by trailing junk: letters, punctuation, a second number, more whitespace, `\0`, high-bit bytes (only the first conversion is performed) | `c15_main_trailing_junk` | [x] |
| C16 | `main` (`.so`, forked) | line-ending shapes: no trailing newline, `"\n"`, `"\r\n"`, `"\n\n"`, digits split by a newline (`"12\n34"` ⇒ only `12`) | `c16_main_line_ending_shapes` | [x] |
| C17 | `main` (`.so`, forked) | digit-string lengths 1…25 with randomised digits, both signs — walks the whole `int`→`long`→overflow ladder | `c17_main_digit_length_ladder` | [x] |
| C18 | `main` (`.so`, forked) | huge digit strings: 100 / 1 000 / 4 095 / 4 096 / 4 097 / 8 191 / 8 192 / 8 193 / 20 000 digits (crosses the 4 KiB stream buffer both builds use, and 8 KiB = `BUFSIZ`), both signs | `c18_main_huge_digit_strings` | [x] |
| C19 | `main` (`.so`, forked) | exact numeric boundaries: `INT_MAX±1`, `INT_MIN±1`, `2^31`, `2^32±1`, `UINT_MAX`, `LONG_MAX±1`, `LONG_MIN±1`, `2^63`, `2^64±1`, 10^19, 10^20, plus randomised 64-bit and 128-bit magnitudes with both signs | `c19_main_numeric_boundaries` | [x] |
| C20 | `main` (`.so`, forked) | stdin is a **pipe** rather than a regular file (unseekable stream, short reads), randomised inputs | `c20_main_stdin_is_a_pipe` | [x] |
| C21 | `main` (`.so`, forked) | whitespace run longer than the stream buffer (4 095 / 4 096 / 4 097 / 8 191 / 8 192 / 8 193 / 40 000 blanks) before the number | `c21_main_whitespace_across_buffer_boundary` | [x] |
| C22 | `main` (`.so`, forked) | embedded `\0` and non-ASCII/UTF-8 bytes before (matching failure) and after (ignored) the digits | `c22_main_nul_and_non_ascii_bytes` | [x] |
| C23 | `main` (`.so`, forked) | return value of the exported `main` compared (must be `0`), for a valid and an invalid input | `c23_main_return_value` | [x] |
| C24 | `driver` + `main` (`.so`) | both entry points driven in the same process, interleaved (`driver`, then `main`, then `driver`) — shared stdout buffer state, and `driver`'s output must not disturb the conversion | `c24_main_and_driver_interleaved`, `c24_driver_state_is_not_sticky` | [x] |
| C25 | `driver` executable (process level) | the real programs (`c_src/build/driver` vs `cargo`-built `driver`): stdin from a regular file, all input classes of C10–C22, comparing stdout **and** exit status | `c25_executables_stdin_file` | [x] |
| C26 | `driver` executable (process level) | same, with stdin from a **pipe** and stdout to a pipe | `c26_executables_stdin_pipe` | [x] |
| C28 | `main` (`.so`, forked) | the exported `main` called **repeatedly in one process** — the composed-pipeline case: glibc's `scanf` keeps the `ungetc` pushback byte and the sticky EOF indicator in the `stdin` FILE, so call *n+1* continues exactly where call *n* stopped. Sub-shapes: several numbers separated by every whitespace kind; a conversion that stops on a non-whitespace byte (`"12x34"`, `"12-34"` — the pushed-back byte is re-read); reading past EOF; unseekable pipe stdin; unreadable stdin; 20-call drains of long randomised token streams; randomised streams of 1–6 tokens with random separators | `c28_repeated_main_multiple_numbers`, `c28_repeated_main_pushback_byte`, `c28_repeated_main_past_eof`, `c28_repeated_main_pipe_stdin`, `c28_repeated_main_long_stream`, `c28_repeated_main_unreadable_stdin`, `c28_repeated_main_randomised_streams` | [x] |
| C29 | `driver` executable (process level) | environment/locale axis: `LC_ALL`, `LC_NUMERIC`, `LANG` set to `C`, `POSIX`, `en_US.UTF-8`, `tr_TR.UTF-8`, `de_DE.UTF-8`, `fr_FR.UTF-8` — the C code never calls `setlocale`, so nothing may change | `c29_executables_ignore_the_locale_environment` | [x] |
| C30 | `driver` executable + `main` (`.so`) | **stream state on fd 0**: how far the conversion reads ahead and where the descriptor is left. Sub-shapes: 1/2/3 runs of the program sharing one open file description (`{ ./driver; ./driver; } < f`), the final `lseek` offset, a co-reader draining the rest (`{ ./driver; cat; } < f`), bytes left queued in an unseekable pipe (inputs of 100/4 000/4 095/4 096/4 097/8 191/8 192/40 000 bytes, straddling glibc's 4 KiB stream buffer), and the exported `main` called 2–4× on one shared descriptor. Randomised token streams too | `c30_stdin_read_ahead_and_offset_are_identical`, `c30b_co_reader_sees_the_same_leftover_bytes`, `c30c_pipe_consumption_is_identical`, `c30d_repeated_main_on_a_shared_descriptor`, `c30_helpers_observe_the_documented_c_behaviour` | [x] |
| C31 | `driver` (`.so`) | **concurrency granularity**: 4 threads × 300 calls in one process. C's `print_hex` is four `printf`s plus a `putchar`, each locking `stdout` on its own, so records may interleave *within* a line; the byte multiset, the byte count and the ability to interleave must all match | `c31_concurrent_driver_calls_have_the_same_granularity` | [x] |
| C32 | `driver` executable (process level) | fd 0 and fd 1 are one read/write descriptor (`./driver <> file`): the read-ahead, the appended output and the final offset all interact | `c32_stdin_and_stdout_on_one_descriptor` | [x] |
| C33 | `driver` + `main` (`.so`), executables | stdout is a **character device** (a real pseudo-terminal): the only configuration in which glibc line-buffers `stdout`. Several `driver` records plus whole-program runs, and the executables under `script(1)`; the terminal's `\n` → `\r\n` translation must appear identically | `c33_stdout_is_a_terminal`, `c33b_executables_under_a_pty` | [x] |
| C34 | `main` (`.so`) | **sticky EOF indicator**: `_IO_new_file_underflow` returns EOF without issuing a `read` once `_IO_EOF_SEEN` is set (C99 requires it). Sub-shapes: the file *grows* between two calls; the descriptor is *rewound* after being drained; both with converted, empty, whitespace-only and non-numeric streams | `c34_eof_indicator_is_sticky` | [x] |
| C35 | `main` (`.so`) | **conversion atomicity**: 2/4/8 threads calling the exported `main` concurrently on a stream of identical tokens. `scanf` holds the stream lock (`flockfile`) for the whole conversion, so a number can never be split between two callers — any other value proves it was | `c35_concurrent_main_never_splits_a_number` | [x] |
| C36 | `main` (`.so`) | **re-entrancy**: a `SIGALRM` handler calls the exported `main` while the outer conversion blocks in `read`. glibc's stream lock is recursive, so the inner conversion consumes the data and the outer then reports end of input; a non-recursive lock would dead-lock (every forked child in the harness has a 30 s watchdog so a dead-lock fails instead of hanging) | `c36_reentrant_main_from_a_signal_handler` | [x] |
| C37 | `main` (`.so`) | **fd 0 descriptor shapes**: pre-positioned with `lseek` (offsets 0–3) × `O_RDONLY`/`O_RDWR`/`O_RDWR|O_APPEND`, a **pty slave** (`st_blksize` 1024, not 4096, so the stream buffer and the number of bytes consumed change), `/dev/null`, `/dev/zero`, and a write-only descriptor (`read` → `EBADF`). Compares stdout, status, the final offset and the file contents | `c37_fd0_descriptor_shapes` | [x] |
| C27 | build configuration | `Cargo.toml` declares no `[features]`, so there is exactly **one** feature combination (the empty one); `cargo check`/`cargo test` are run both with `--no-default-features` and with the default feature set, which are the same configuration. Every row above is additionally re-run against a Rust `.so` built with `opt-level=0`/`debug-assertions=on`/`overflow-checks=on` **and** one built with `opt-level=3`/`panic=abort`/`overflow-checks=off` (mirroring `[profile.release]`), and with the test binaries themselves built in both cargo profiles | `RUST_SO_PROFILE` env switch + `scripts/verify_all.sh` | [x] |

## Deliberately not emulated

What is left after the rows above are green is a small, precisely bounded set:
behaviours that are only reachable by a *C host* that shares glibc's own `FILE`
objects or exit machinery with the library, or by making an internal allocation
fail. Matching them would mean calling `printf`/`scanf` from Rust instead of
translating them, so they are documented rather than emulated — each with its
reproduction, so the boundary is explicit instead of accidental.

Measured scope (host programs that `dlopen` both `.so`s and call the exports in
scripted sequences, `$TMPDIR/probe`, `$TMPDIR/h`, `$TMPDIR/audit2`): **every
sequence that only calls `driver`/`main` agrees exactly** — 196/196 mixed
sequences, 2568/2568 repeated-`main` comparisons, 224/224 rewound-descriptor and
168/168 growing-file shapes, 20/20 concurrency trials at 2–16 threads, and the
re-entrant-handler case. The divergences below all require the host itself to use
glibc stdio, its own `atexit`/`dlclose`, or to abandon `exit`.

| behaviour | reproduction | C | this translation |
|-----------|--------------|---|------------------|
| `stdout` is *flushed at exit* by glibc, so a host that mixes its own `printf`/`write` with calls into the library sees a different **ordering**, and a host that `_exit`s/`abort`s/`fork`s (or dies of a signal) after the call sees the buffered line **dropped/duplicated** | a C host doing `printf("A"); driver(1);` then `exit`/`_exit`/`fork`/`abort` | `A01000000\n`; line lost on `_exit`/`abort`; duplicated across `fork` | the record reaches fd 1 when its `'\n'` is written (`01000000\nA`; never lost, never duplicated). Ordering could only be matched by writing into glibc's `stdout` FILE itself. The program's own byte stream is identical (rows C8/C9/C25/C26/C33) |
| the conversion shares glibc's `stdin` FILE with the host, so a host `getchar()`/`ungetc()`/`feof(stdin)`/`fflush(stdin)` interleaves with it | a C host doing `getchar(); main();` on `"1 2 3"` | second value `2` | `0` — the host's `getchar` and the translation's stream are separate readers of fd 0. The *descriptor-level* behaviour (bytes consumed, final offset, leftovers for the next reader, sticky EOF) **is** matched: rows C30/C34 |
| the exit-time rewind is registered with `atexit`, which binds it to this DSO and puts it in the user handler list; glibc runs `_IO_cleanup` from `__libc_atexit`, i.e. **after** all user handlers and **never** at `dlclose` | a host doing `main(); dlclose(lib); lseek(0,0,SEEK_CUR)`, or registering its own `atexit` handler *before* the first call | rewind happens at process exit only, after the host's handlers | the rewind happens at `dlclose`, and before a host handler registered earlier. Using `__cxa_atexit(.., NULL)` would fix the ordering but would leave a dangling handler after `dlclose`; the final offset at process exit is identical either way |
| `vfscanf` copies the whole digit run into a heap scratch buffer, so a huge run fails the conversion when that allocation fails | `( ulimit -v 24000; ./driver ) < 16-MiB-of-nines` | `00000000` (input failure) | `ffffffff` — the translation parses in O(1) memory and never allocates, so it cannot fail there. Without an artificial `RLIMIT_AS` both agree (rows E7/C18 cover 5 000- and 20 000-digit runs and a 1 MiB run) |

The stream *buffer* allocation, by contrast, is handled: when it fails the
translation falls back to a one-byte buffer, matching glibc's `_shortbuf`
fallback, instead of aborting.
