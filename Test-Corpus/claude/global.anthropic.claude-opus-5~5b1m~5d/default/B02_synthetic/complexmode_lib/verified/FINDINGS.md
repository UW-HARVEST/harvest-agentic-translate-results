# FINDINGS.md — divergences found by the differential suite

The C in `c_src/` is ground truth; everything below was fixed on the Rust side.

## 1. `multiply_with_log(a, b, NULL)` aborted instead of segfaulting (FIXED)

**Row:** `ERRORS.md` E4. **Test:** `err_e4_multiply_with_log_null_outparam`.

`c_src/src/lib.c:59` has no null check on the out-param:

```c
int multiply_with_log(int a, int b, char** log_msg) {
    *log_msg = create_result_string("multiply", a * b);
```

so `multiply_with_log(6, 7, NULL)` stores through a null pointer. The compiled C
dies with **SIGSEGV (11)**.

The original translation wrote `*log_msg = create_result_string(...)`. In a
**release** build that is a bare machine store and also faults with SIGSEGV — so
the release-only test run passed. But under `debug_assertions` rustc instruments
raw-pointer place assignments with a null check, so the debug build instead
panicked:

```
thread '<unnamed>' panicked at src/lib.rs:119:9:
null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

i.e. **SIGABRT (6)** — a different signal, a different exit status, and extra
bytes on stderr. Fixed by performing the store and the read-back with
`core::ptr::write` / `core::ptr::read`, which emit the bare accesses and are not
instrumented, so debug and release now both fault exactly like the C:

```rust
ptr::write(log_msg, create_result_string(c"multiply".as_ptr(), a.wrapping_mul(b)));
if ptr::read(log_msg).is_null() { return 0; }
```

Verified: `cargo test` and `cargo test --release` both report signal 11 for both
libraries.

## Harness bugs that were masking coverage (fixed, no library change)

These did not indicate a translation defect, but each one would have let a real
divergence slip through, so they are recorded:

1. **The tests loaded the wrong cdylib.** `rust_so_path()` originally preferred
   `target/release/`, so `cargo test` (debug) could silently exercise the
   *release* `.so` and never see debug-only codegen — exactly the class of bug
   above. It now derives the path from `current_exe()`'s profile directory, and
   `d7_test_binary_loads_the_matching_profile_cdylib` enforces it.

2. **The OOM injection was a no-op in release builds.** The heap-drain loop for
   rows E1/E3/E10/E12 was written as `let p = malloc(n); if p.is_null() {...}`.
   LLVM removed the whole loop as an unused allocation and folded `is_null()` to
   `false`; `/proc/self/statm` confirmed the address space never grew. The rows
   would have "passed" without ever reaching the C's allocation-failure branch.
   Fixed by storing into each block and passing every pointer through
   `std::hint::black_box`, and by adding an explicit in-child probe that verifies
   `malloc` really fails (the child exits 3/4/5 otherwise, which the parent
   asserts on).

3. **Row E12 was not actually inducible by a single drain.** In a multi-threaded
   process glibc serves the drain from a secondary arena, where a failed
   `malloc(40)` does *not* imply a failed `malloc(64)`. The child now drains both
   chunk classes and releases two 48-byte blocks from deep inside the heap, so
   `malloc(sizeof(Result))` succeeds from tcache while `malloc(64)` still fails —
   which is precisely the state `lib.c:131` needs.

   As a side effect this row is also a check on `sizeof(Result)`: the injected
   state only serves requests in the 48-byte chunk class, so if the Rust
   `#[repr(C)] struct Result` were a different size than the C's 40 bytes, the
   Rust child would print `Failed to allocate result tracker` instead of
   `Log message creation failed` and the test would fail.

4. **`printf("%s", marker)` hung under heap exhaustion**, because glibc's
   `vfprintf` allocates working buffers. Markers are now emitted with `write(2)`
   and the stdout `FILE` buffer is warmed up before the drain, so the library's
   own `printf`/`puts` can still run.

## Confirmed-faithful oddities (replicated, not "fixed")

* `complexmode` mode 4 checks `check_permissions(0644, 0100)`, which is
  `0644 & 0100 == 0 != 0100` — **false**. The `value1*value2+value3` branch is
  therefore dead and the result is always `value1+value2+value3`. `c33_...`
  asserts the C really takes the else-branch before comparing, so the Rust is
  held to the same (surprising) behaviour.
* `copy_and_sum` with `count == 0` returns `0`, because glibc `malloc(0)` is
  non-NULL. With any *negative* `count` the `int` sign-extends to a huge
  `size_t`, `malloc` fails, and it returns `-1` with `Memory allocation failed`.
* `create_result_string(NULL, v)` does not check for NULL; glibc `snprintf`
  prints the literal `(null)`. Replicated by forwarding to the same `snprintf`.
* `safe_add` returns `0` (not an error code) when permissions are insufficient,
  which is indistinguishable from a legitimate `a + b == 0`. Replicated.
* Signed-overflow (UB in C) is compiled by GCC as two's-complement wraparound;
  the translation uses `wrapping_add`/`wrapping_mul`, which matches across
  thousands of randomized overflowing inputs (rows C7, C15, C21, C29, C31–C34).
