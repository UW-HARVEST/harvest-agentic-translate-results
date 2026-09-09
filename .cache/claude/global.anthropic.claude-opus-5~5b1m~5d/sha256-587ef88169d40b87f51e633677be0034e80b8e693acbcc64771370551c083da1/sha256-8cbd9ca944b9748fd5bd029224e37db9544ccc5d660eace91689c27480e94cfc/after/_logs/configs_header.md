# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

The mirror of `ERRORS.md` for VALID input. Derived mechanically from the C
sources and the public headers in `c_src/libsodium/include/sodium/`: every
runtime option / mode / flag the public API can set, every distinct input
SHAPE the code special-cases (sizes, block/rate boundaries, empty / one / many,
chunk splits, counter values, primitive selection), and the FULL set of public
entry points — including the lowest-level ones (`*_afternm`, `*_beforenm`,
`*_detached`, `*_xor_ic`, the NaCl zero-padded forms, `init`/`update`/`final`
streaming APIs, `*_ll`), not just the one-shot convenience wrappers.

Each row is one combination the C code treats differently. Rows are exercised
with MANY randomized inputs from a fixed seed (`tests/common/mod.rs::Rng`), not
a single hand-picked value.

Non-deterministic entry points (`*_keygen`, `*_keypair`, `*_random`,
`randombytes_buf`, `crypto_box_seal`, `crypto_secretstream_*_init_push`,
`crypto_pwhash_str*`, ...) are made comparable by installing one deterministic
`randombytes_implementation` into BOTH libraries via
`randombytes_set_implementation` (`tests/common/mod.rs::install_det_random`,
plus `det_reseed` to replay the same stream for each side), so both observe an
identical random stream and their full outputs can be compared byte-for-byte.

The `covered by` column names the `#[test]` function(s) that exercise the row.
Regenerate this file with `python3 _logs/gen_tables.py`.

| # | entry point(s) | configuration (options set + input shape) | [ ] | covered by |
|---|----------------|--------------------------------------------|-----|------------|
