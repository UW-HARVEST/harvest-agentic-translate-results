# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Derived mechanically from the C sources in `c_src/libsodium` by grepping every
`return -1`, `return NULL`, failure-`return 0`, `ARGON2_*` error enum,
`sodium_misuse()`, `abort()`, `assert(`, explicit range/size/overflow check,
null check and `errno =` site, then reading the surrounding code to state the
exact triggering condition.

**Response classes.** libsodium rejects input in three observably different
ways, and a differential test must distinguish them:

1. **return value** — `-1`, `NULL`, `0`-as-failure. Compared directly.
2. **`errno` side effect** — `ERANGE`, `EINVAL`, `ENOMEM`, `ENOSYS`, `EPERM`.
   Both libraries share the calling thread's `errno`, so the harness zeroes it,
   calls, and compares (`tests/common/mod.rs::errno`).
3. **process death** — `sodium_misuse()` (which always ends in `abort()`),
   `assert()`, `_out_of_bounds()` (`raise(SIGSEGV)`). Compared by re-executing
   the test binary in a child process against one library at a time and
   comparing the child's exit code / terminating signal
   (`tests/common/mod.rs::diff_abort_case`).

**Assertions are LIVE in the reference C build.** `c_src/build` is configured
with an empty `CMAKE_BUILD_TYPE`, so no `-DNDEBUG` is passed and every C
`assert()` is compiled in. Rows whose expected result is an `assert` failure are
therefore real, reachable SIGABRTs that the Rust must reproduce. (Two genuine
divergences were found this way: `crypto_generichash_blake2b_final` with
`outlen > 255`, and the `randombytes` chunking wrapper with `size == 0`.)

**Build configuration.** The reference C build defines no `HAVE_*` macros, so
`HAVE_MLOCK`, `HAVE_MPROTECT`/`HAVE_PAGE_PROTECTION`, `HAVE_ALIGNED_MALLOC`,
`HAVE_AMD64_ASM`, `HAVE_MADVISE`, `HAVE_GETPID`, `HAVE_TMMINTRIN_H`/
`HAVE_WMMINTRIN_H` and `HAVE_ARMCRYPTO` are all **off**. Rows guarded by those
macros are configuration dependent; their tests assert C/Rust *agreement*
rather than a hard-coded value, so they stay correct either way.

The `covered by` column names the `#[test]` function(s) that exercise the row.
Regenerate this file with `python3 _logs/gen_tables.py`.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|------------|
