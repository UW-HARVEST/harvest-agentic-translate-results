# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically from:

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# Rust
cd translation && cargo build --release

nm -D --defined-only c_src/build/libdriver.so          | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.txt
diff /tmp/c.txt /tmp/r.txt
```

`tests/phase_d_symbols.rs` re-runs exactly this comparison as a test, so the
table below cannot silently rot.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | origin | Rust definition site |
|---|--------|---------|-----------|--------|----------------------|
| 1 | `create_task_manager`  | T | T | `c_src/src/task_manager.c:32` | `src/task_manager.rs` `#[unsafe(no_mangle)] extern "C"` |
| 2 | `add_task`             | T | T | `c_src/src/task_manager.c:53` | `src/task_manager.rs` |
| 3 | `print_tasks`          | T | T | `c_src/src/task_manager.c:67` | `src/task_manager.rs` |
| 4 | `destroy_task_manager` | T | T | `c_src/src/task_manager.c:74` | `src/task_manager.rs` |
| 5 | `initialize_logger`    | T | T | `c_src/src/logger.c:33`       | `src/logger.rs` |
| 6 | `log_info`             | T | T | `c_src/src/logger.c:47`       | `src/logger.rs` |
| 7 | `log_warning`          | T | T | `c_src/src/logger.c:53`       | `src/logger.rs` |
| 8 | `log_error`            | T | T | `c_src/src/logger.c:59`       | `src/logger.rs` |
| 9 | `finalize_logger`      | T | T | `c_src/src/logger.c:65`       | `src/logger.rs` |
| 10 | `driver`              | T | T | `c_src/src/driver.c:32`       | `src/driver.rs` |

**`diff` result: empty — 0 symbols missing from the Rust `.so`.**

There are no macro-generated exports in this library: all three C translation
units define plain, non-`static` functions and no `static` object has external
linkage (`logger.c`'s `log_file` is `static`, so it is correctly *not* exported
by either build; the Rust equivalent is the private `static mut LOG_FILE`).

No symbol required a newly added wrapper and no C translation unit was missing
from the Rust crate — `task_manager.c`, `logger.c` and `driver.c` map 1:1 onto
`src/task_manager.rs`, `src/logger.rs` and `src/driver.rs`.

## Undefined (imported) symbols

The C `.so` imports 14 glibc functions/objects:

```
atoi fclose fopen fprintf free fwrite getenv malloc printf puts stderr
strchr strlen strncpy
```

(`fwrite`/`puts` are the compiler's own strength reductions of `fprintf`/
`printf` with constant formats — not calls written in the source.)

The Rust `.so` imports the same set plus the Rust-runtime/libgcc set that any
`cdylib` pulls in (`memcpy`, `memset`, `mmap64`, `_Unwind_*`, `pthread_key_*`,
…).  Checked with `nm -D -u`: **every** undefined symbol in the Rust `.so`
resolves to `libc.so.6` or `libgcc_s.so.1`; `ldd` lists no other dependency, so
there are 0 unresolved non-libc symbols.

Notably the translation deliberately imports and calls the *real* `malloc`,
`free`, `getenv`, `atoi`, `strncpy`, `strchr`, `strlen`, `fopen`, `fclose`,
`fprintf` and `printf` (`src/cbind.rs`) instead of reimplementing them, which is
what makes `atoi` overflow behaviour, `printf` `%s`/`%d` formatting, stdio
buffering and allocator identity identical by construction.

`src/cstdio.rs`, `src/cutil.rs` and `src/stdio_stream.rs` are not reachable from
`src/lib.rs`'s module tree and therefore contribute no code to the `.so`; they
are unused leftovers of the translation process.  They were left in place
because removing them changes nothing observable, but note that the `c_atoi`,
`strncpy` and `StdioStream` reimplementations they contain are **not** what the
library uses — the real glibc functions are, via `src/cbind.rs`.

## Build caveat

`cargo test` builds the test harnesses only; it never rebuilds the `cdylib`
artifact at `target/release/libdriver.so`.  Testing therefore requires an
explicit `cargo build --release` first, and `tests/common/mod.rs` asserts that
each `.so` is newer than its sources so a stale artifact cannot be mistaken for
a passing verification.
