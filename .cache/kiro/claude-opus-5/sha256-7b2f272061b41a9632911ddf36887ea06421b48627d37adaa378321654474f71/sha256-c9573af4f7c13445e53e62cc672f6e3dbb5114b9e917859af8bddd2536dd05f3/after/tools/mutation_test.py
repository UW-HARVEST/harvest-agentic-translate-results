#!/usr/bin/env python3
"""Mutation test: prove the differential suite actually DETECTS divergence.

For each entry below we inject a small, semantics-changing bug into the Rust
translation, rebuild the `.so`, run the named test target, and require that the
target turns RED. A mutation that goes undetected marks an untested part of the
surface.

Every file is restored afterwards (and the build is left clean).
"""
import os, shutil, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "translation", "src")

# (label, relative source file, needle, replacement, cargo test target)
MUTATIONS = [
    ("salsa core: quarter-round rotation 7 -> 8",
     "crypto_core/salsa.rs", "rotl32(x0.wrapping_add(x12), 7)",
     "rotl32(x0.wrapping_add(x12), 8)", "t03_core"),
    ("keccak1600: theta rotation off by one",
     "crypto_core/keccak1600.rs", "let da = $cu ^ rotl64($ce, 1);",
     "let da = $cu ^ rotl64($ce, 2);", "t03_core"),
    ("hchacha20: sigma constant word 0",
     "crypto_core/hchacha20.rs", "x0 = 0x61707865;", "x0 = 0x61707866;", "t03_core"),
    ("sha256: wrong round constant",
     "crypto_hash/sha256.rs", "0x428a2f98", "0x428a2f99", "t04_hash_xof"),
    ("sha512: wrong round constant",
     "crypto_hash/sha512.rs", "0x428a2f98d728ae22", "0x428a2f98d728ae23", "t04_hash_xof"),
    ("shake128: wrong domain byte",
     "crypto_xof/shake128.rs", "crypto_xof_shake128_DOMAIN_STANDARD: u8 = 0x1F",
     "crypto_xof_shake128_DOMAIN_STANDARD: u8 = 0x1E", "t04_hash_xof"),
    ("shake256: wrong rate",
     "crypto_xof/shake256.rs", "SHAKE256_RATE: usize = 136", "SHAKE256_RATE: usize = 128",
     "t04_hash_xof"),
    ("blake2b: sigma table row 1 corrupted",
     "crypto_generichash/blake2b.rs", "[14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3]",
     "[14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 3, 5]", "t05_mac"),
    ("poly1305: wrong r clamp mask",
     "crypto_onetimeauth/poly1305.rs", "0x3ffff03", "0x3ffff07", "t05_mac"),
    ("siphash24: wrong rotation",
     "crypto_shorthash/siphash24.rs", "rotl64($v1, 13)", "rotl64($v1, 14)", "t05_mac"),
    ("codecs: hex digit table shifted",
     "sodium_codecs.rs", "(c as u32).wrapping_sub(10)", "(c as u32).wrapping_sub(11)",
     "t02_utils_codecs"),
    ("ipcrypt: prefix shift wrong",
     "crypto_ipcrypt.rs", "(*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 7)",
     "(*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 6)", "t10_kdf_kem_ipcrypt"),
    ("aegis128l: mac length block low word",
     "crypto_aead/aegis128l.rs", "        w0: b as u32,", "        w0: (b as u32) ^ 1,",
     "t07_secretbox_aead"),
    ("aegis256: mac length block low word",
     "crypto_aead/aegis256.rs", "        w0: b as u32,", "        w0: (b as u32) ^ 1,",
     "t07_secretbox_aead"),
    ("sc25519_reduce: group order table byte wrong",
     "crypto_core/ed25519_ref10/sc.rs",
     "0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7,",
     "0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf6,",
     "t08_ecc"),
    ("mlkem768: wrong modulus",
     "crypto_kem/mlkem768.rs", "MLKEM768_Q: i32 = 3329", "MLKEM768_Q: i32 = 3331",
     "t10_kdf_kem_ipcrypt"),
    ("argon2: wrong block size",
     "crypto_pwhash/argon2/argon2_core.rs", "ARGON2_BLOCK_SIZE: usize = 1024",
     "ARGON2_BLOCK_SIZE: usize = 512", "t11_pwhash"),
    # --- round 2: the families not probed above ---
    ("crypto_verify_n: constant-time compare shifted",
     "crypto_verify.rs", "d = ((d >> 13) ^", "d = ((d >> 12) ^", "t02_utils_codecs"),
    ("sodium_compare: gt/eq combination wrong",
     "sodium_utils.rs", "(gt as c_int) + (gt as c_int) + (eq as c_int) - 1",
     "(gt as c_int) + (eq as c_int) - 1", "t02_utils_codecs"),
    ("hkdf expand: counter starts at 2",
     "crypto_kdf/hkdf_sha256.rs", "let mut counter: c_uchar = 1u8;",
     "let mut counter: c_uchar = 2u8;", "t10_kdf_kem_ipcrypt"),
    ("hkdf-sha512 expand: counter starts at 2",
     "crypto_kdf/hkdf_sha512.rs", "let mut counter: c_uchar = 1u8;",
     "let mut counter: c_uchar = 2u8;", "t10_kdf_kem_ipcrypt"),
    ("kdf_blake2b: CONTEXTBYTES wrong",
     "crypto_kdf/blake2b.rs", "crypto_kdf_blake2b_CONTEXTBYTES: usize = 8;",
     "crypto_kdf_blake2b_CONTEXTBYTES: usize = 7;", "t10_kdf_kem_ipcrypt"),
    ("kx: primitive/subkey personalisation string",
     "crypto_kx.rs", "b\"x25519blake2b\\0\"", "b\"x25519blake2c\\0\"", "t09_sign_box_kx"),
    ("scrypt: STRHASHBYTES wrong",
     "crypto_pwhash/scrypt/crypto_scrypt_common.rs", "const STRHASHBYTES: usize = 32;",
     "const STRHASHBYTES: usize = 31;", "t11_pwhash"),
    ("chacha20: sigma constant",
     "crypto_stream/chacha20.rs", "0x61707865", "0x61707866", "t06_stream"),
    ("salsa20 stream: nonce/counter block layout",
     "crypto_stream/salsa20.rs", "crypto_stream_salsa20_NONCEBYTES: usize = 8;",
     "crypto_stream_salsa20_NONCEBYTES: usize = 7;", "t06_stream"),
    ("secretbox xsalsa20poly1305: poly1305 offset wrong",
     "crypto_secretbox/xsalsa20poly1305.rs",
     "crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);",
     "crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c.add(1));",
     "t07_secretbox_aead"),
    ("randombytes deterministic: nonce personalisation",
     "randombytes.rs", "b'D', b'R', b'G'", "b'D', b'R', b'H'", "t12_randombytes"),
    ("sign ed25519: scalar clamp 248 -> 249",
     "crypto_sign/ed25519.rs", "h[0] &= 248;", "h[0] &= 249;", "t09_sign_box_kx"),
    ("sign ed25519: sk_to_curve25519 clamp wrong",
     "crypto_sign/ed25519.rs", "*sk.add(0) &= 248;", "*sk.add(0) &= 249;", "t09_sign_box_kx"),
    ("box beforenm: hsalsa20 zero input changed",
     "crypto_box/curve25519xsalsa20poly1305.rs", "static zero: [u8; 16] = [0; 16];",
     "static zero: [u8; 16] = [1; 16];", "t09_sign_box_kx"),
]


def run(cmd, **kw):
    return subprocess.run(cmd, shell=True, cwd=os.path.join(ROOT, "translation"),
                          capture_output=True, text=True, **kw)


def main():
    detected, undetected, skipped = [], [], []
    for label, rel, needle, repl, target in MUTATIONS:
        path = os.path.join(SRC, rel)
        if not os.path.exists(path):
            skipped.append((label, "file missing"))
            print(f"SKIP           {label}: no {rel}")
            continue
        orig = open(path).read()
        if needle not in orig:
            skipped.append((label, "needle missing"))
            print(f"SKIP           {label}: needle not found in {rel}")
            continue
        open(path, "w").write(orig.replace(needle, repl, 1))
        try:
            b = run("timeout 600 cargo build --release")
            if b.returncode != 0:
                skipped.append((label, "does not compile"))
                print(f"SKIP           {label}: mutated source does not compile")
                continue
            t = run(f"timeout 600 cargo test --release --test {target}")
            if t.returncode == 0:
                undetected.append(label)
                print(f"NOT DETECTED   {label}  ({target})")
            else:
                detected.append(label)
                print(f"detected       {label}  ({target})")
        finally:
            open(path, "w").write(orig)
    run("timeout 600 cargo build --release")
    print()
    print(f"detected: {len(detected)}   NOT detected: {len(undetected)}   skipped: {len(skipped)}")
    for s in skipped:
        print("  skipped:", s)
    if undetected:
        print("  UNDETECTED:", undetected)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
