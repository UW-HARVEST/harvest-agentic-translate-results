# ERRORS.md — Error-surface table (aggregate)

Mechanically derived from the C source: every distinct rejection
(return -1 / NULL / sentinel, range check, tag/point validation,
`sodium_misuse()`/abort). Each row has a differential test (Phase C)
asserting C and Rust return the SAME error/sentinel.

Per-family tables live in `docs/<family>_ERRORS.md`. Total rejection
rows across all families: **137**.

| family | rows | doc |
|--------|------|-----|
| aead | 21 | `docs/aead_ERRORS.md` |
| authmac | 24 | `docs/authmac_ERRORS.md` |
| hashing | 15 | `docs/hashing_ERRORS.md` |
| kemip | 7 | `docs/kemip_ERRORS.md` |
| pubkey | 23 | `docs/pubkey_ERRORS.md` |
| pwkdf | 22 | `docs/pwkdf_ERRORS.md` |
| sodiumutils | 22 | `docs/sodiumutils_ERRORS.md` |
| streamcore | 3 | `docs/streamcore_ERRORS.md` |

Notes: `sodium_misuse()`→`abort()` paths are process-terminating (not
in-band return codes); where reachable they are verified via forked
subprocess (e.g. streamcore chacha20_ietf ic-overflow) or documented as
unreachable in this portable build (e.g. AES-GCM unavailable, 64-bit
MESSAGEBYTES_MAX). All in-band -1/sentinel paths have executed tests.
