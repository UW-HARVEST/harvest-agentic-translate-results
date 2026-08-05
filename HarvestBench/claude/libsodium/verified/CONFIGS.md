# CONFIGS.md — Configuration-surface table (aggregate)

Mechanically derived from the C source + public headers: every runtime
option/mode/flag, every special-cased input shape (sizes, block
boundaries, empty/one/many, byte order), and the FULL set of public entry
points INCLUDING low-level ones (`_detached`, `_init/_update/_final`,
`_xor_ic`, `beforenm/afternm`, deterministic `_seed_keypair/_enc_deterministic`).

Each row is exercised by a Phase B differential test with MANY randomized
inputs (fixed seed) asserting byte-for-byte C==Rust. Per-family tables live
in `docs/<family>_CONFIGS.md`. Total config rows: **209** (all checked).

| family | rows | doc |
|--------|------|-----|
| aead | 24 | `docs/aead_CONFIGS.md` |
| authmac | 18 | `docs/authmac_CONFIGS.md` |
| hashing | 28 | `docs/hashing_CONFIGS.md` |
| kemip | 15 | `docs/kemip_CONFIGS.md` |
| pubkey | 33 | `docs/pubkey_CONFIGS.md` |
| pwkdf | 17 | `docs/pwkdf_CONFIGS.md` |
| sodiumutils | 43 | `docs/sodiumutils_CONFIGS.md` |
| streamcore | 31 | `docs/streamcore_CONFIGS.md` |
