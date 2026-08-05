# Feature / build-configuration enumeration

## Cargo features
`Cargo.toml` has **no `[features]` section**, and the source contains **zero**
`#[cfg(feature = "...")]` gates (`grep -rc 'cfg(feature' src/` → 0). Therefore
there is exactly ONE Cargo build configuration.

- `cargo check` (default) — OK
- `cargo check --no-default-features` — OK (identical; no default features to drop)

The cross-product of feature combinations is a single element: `{}` (no features).
All Phase B / Phase C tests run under this one configuration, which is the only
one that exists.

## C build configuration
`c_src/CMakeLists.txt` defines **no `HAVE_*` macros** — every `#ifdef HAVE_*`
in libsodium selects the portable fallback (equivalent to `configure
--disable-asm`). This matches the Rust translation, which is likewise the
portable/reference implementation. There is a single C configuration too.

## Runtime configuration surface
Although there are no *build-time* features, the library has a rich *runtime*
option surface (base64 variants, argon2i vs argon2id, secretstream tags, IETF
vs original AEAD nonces, key/output lengths, deterministic vs randomized entry
points, etc.). Those runtime axes are enumerated and tested in
`CONFIGS.md` / `docs/<family>_CONFIGS.md` — they are the real "configuration
combinations" this library branches on.
