# streamcore error paths

Rejections found in the C source for the STREAM CIPHERS + CORE family.

Notes on the family's error model:
- The salsa20/salsa2012/salsa208 stream functions and all the `crypto_core_*`
  primitives perform NO input validation: they always return `0` and cannot
  fail on any (correctly-sized) input. Passing a wrong-sized buffer is
  undefined behavior in C (out-of-bounds read/write), not a defined rejection,
  so it is not a testable "same error code" path and is excluded.
- The chacha20 family guards message length against `MESSAGEBYTES_MAX` via
  `sodium_misuse()`, which calls `abort()`. On a 64-bit platform
  `crypto_stream_chacha20_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX == UINT64_MAX`,
  so the `clen/mlen > MESSAGEBYTES_MAX` branches are unreachable through the FFI
  `u64` argument (nothing exceeds UINT64_MAX) and are not testable.
- The only reachable, deterministic rejection is the IETF initial-counter
  overflow check, which is exercised in `tests/streamcore.rs`
  (`err_ietf_xor_ic_overflow_aborts_both`) using `fork()` so the `abort()` does
  not terminate the test runner.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | crypto_stream_chacha20_ietf_xor_ic | `ic > (2^32 - ceil(mlen/64))` (initial counter would overflow the 32-bit block counter) | `sodium_misuse()` -> `abort()` (process terminates with SIGABRT) |
| 2 | crypto_stream_chacha20 / _xor / _xor_ic | `clen`/`mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` | `sodium_misuse()` -> `abort()`. Unreachable on 64-bit (MAX == UINT64_MAX); documented for completeness. |
| 3 | crypto_stream_chacha20_ietf / _ietf_xor | `clen`/`mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` (== `64*2^32`) | `sodium_misuse()` -> `abort()`. Not reachable via the exercised message sizes. |
