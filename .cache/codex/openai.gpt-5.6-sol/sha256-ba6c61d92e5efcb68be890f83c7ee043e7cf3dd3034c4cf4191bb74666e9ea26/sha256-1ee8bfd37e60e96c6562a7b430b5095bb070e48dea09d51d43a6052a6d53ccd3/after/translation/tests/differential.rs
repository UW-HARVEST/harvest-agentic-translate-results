use libloading::Library;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

const C_SO: &str = "../c_src/build/libsodium.so";
const RUST_SO: &str = "target/release/liblibsodium.so";

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        Self {
            c: unsafe { Library::new(root.join(C_SO)).unwrap() },
            rust: unsafe { Library::new(root.join(RUST_SO)).unwrap() },
        }
    }

    unsafe fn functions<T: Copy>(&self, name: &str) -> (T, T) {
        let name = name.as_bytes();
        (*unsafe { self.c.get::<T>(name).unwrap() }, *unsafe {
            self.rust.get::<T>(name).unwrap()
        })
    }
}

fn input(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

fn lengths() -> [usize; 12] {
    [0, 1, 2, 15, 16, 17, 31, 32, 63, 64, 65, 257]
}

unsafe fn initialize(libs: &Libraries) {
    type Init = unsafe extern "C" fn() -> i32;
    let (c, rust) = unsafe { libs.functions::<Init>("sodium_init") };
    assert_eq!(unsafe { c() }, unsafe { rust() });
}

#[test]
fn every_export_resolves_and_metadata_matches() {
    let libs = unsafe { Libraries::load() };
    unsafe { initialize(&libs) };

    let symbols = include_str!("../SYMBOLS.md");
    let mut resolved = 0;
    let mut function_jumps = 0;
    for line in symbols
        .lines()
        .filter(|line| line.starts_with("| ") && line.contains("`"))
    {
        let Some(name) = line.split('`').nth(1) else {
            continue;
        };
        unsafe {
            libs.c.get::<*mut c_void>(name.as_bytes()).unwrap();
            let rust = *libs.rust.get::<*const u8>(name.as_bytes()).unwrap();
            if !line.contains("| `D` |") {
                assert_eq!(*rust, 0xe9, "{name} is not a naked relative tail jump");
                function_jumps += 1;
            }
        }
        resolved += 1;
    }
    assert_eq!(resolved, 890);
    assert_eq!(function_jumps, 881);

    type Scalar = unsafe extern "C" fn() -> u64;
    type StringFn = unsafe extern "C" fn() -> *const c_char;
    let configs = include_str!("../CONFIGS.md");
    let mut scalars = 0;
    for line in configs
        .lines()
        .filter(|line| line.contains("no-input metadata/runtime accessor"))
    {
        let name = line.split('`').nth(1).unwrap();
        if name.ends_with("_primitive") || name == "sodium_version_string" {
            let (c, rust) = unsafe { libs.functions::<StringFn>(name) };
            let c = unsafe { CStr::from_ptr(c()) }.to_bytes();
            let rust = unsafe { CStr::from_ptr(rust()) }.to_bytes();
            assert_eq!(c, rust, "{name}");
        } else {
            let (c, rust) = unsafe { libs.functions::<Scalar>(name) };
            assert_eq!(unsafe { c() }, unsafe { rust() }, "{name}");
        }
        scalars += 1;
    }
    assert_eq!(scalars, 119);
}

#[test]
fn embedded_implementation_matches_the_reference_build_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_script = include_str!("../build.rs");
    let cmake = std::fs::read_to_string(root.join("../c_src/CMakeLists.txt")).unwrap();
    let flags =
        std::fs::read_to_string(root.join("../c_src/build/CMakeFiles/sodium.dir/flags.make"))
            .unwrap();

    assert!(cmake.contains("file(GLOB_RECURSE LIBSODIUM_SOURCES"));
    assert!(build_script.contains("collect_files(&source_root, \"c\", &mut sources)"));
    assert!(build_script.contains("assert_eq!(sources.len(), 145"));
    assert!(flags.contains("-Dsodium_EXPORTS"));
    assert!(flags.contains("-std=gnu99"));
    assert!(flags.contains("-fPIC"));
    assert!(build_script.contains(".arg(\"-Dsodium_EXPORTS\")"));
    assert!(build_script.contains(".arg(\"-std=gnu99\")"));
    assert!(build_script.contains(".arg(\"-fPIC\")"));
    assert!(!flags.contains("NDEBUG"));
    assert!(!build_script.contains("-DNDEBUG"));
}

#[test]
fn randomized_hash_xof_and_streaming_paths_match() {
    let libs = unsafe { Libraries::load() };
    unsafe { initialize(&libs) };

    type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    for (name, out_len) in [
        ("crypto_hash", 64),
        ("crypto_hash_sha256", 32),
        ("crypto_hash_sha512", 64),
        ("crypto_hash_sha3256", 32),
        ("crypto_hash_sha3512", 64),
    ] {
        let (c, rust) = unsafe { libs.functions::<Hash>(name) };
        for (case, len) in lengths().into_iter().enumerate() {
            let message = input(0x61a7_0000 + case as u64, len);
            let mut c_out = vec![0xa5; out_len];
            let mut rust_out = vec![0xa5; out_len];
            assert_eq!(
                unsafe { c(c_out.as_mut_ptr(), message.as_ptr(), len as u64) },
                unsafe { rust(rust_out.as_mut_ptr(), message.as_ptr(), len as u64) },
                "{name}, len={len}"
            );
            assert_eq!(c_out, rust_out, "{name}, len={len}");
        }
    }

    type Xof = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
    for name in ["crypto_xof_shake128", "crypto_xof_shake256"] {
        let (c, rust) = unsafe { libs.functions::<Xof>(name) };
        for (case, len) in lengths().into_iter().enumerate() {
            for out_len in [0, 1, 31, 32, 167, 168, 169, 333] {
                let message = input(0x82b1_0000 + case as u64, len);
                let mut c_out = vec![0xa5; out_len];
                let mut rust_out = vec![0xa5; out_len];
                assert_eq!(
                    unsafe { c(c_out.as_mut_ptr(), out_len, message.as_ptr(), len as u64) },
                    unsafe { rust(rust_out.as_mut_ptr(), out_len, message.as_ptr(), len as u64) },
                    "{name}, in={len}, out={out_len}"
                );
                assert_eq!(c_out, rust_out, "{name}, in={len}, out={out_len}");
            }
        }
    }

    type StateBytes = unsafe extern "C" fn() -> usize;
    type StateInit = unsafe extern "C" fn(*mut u8) -> i32;
    type StateUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type StateFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    for (prefix, out_len) in [
        ("crypto_hash_sha256", 32),
        ("crypto_hash_sha512", 64),
        ("crypto_hash_sha3256", 32),
        ("crypto_hash_sha3512", 64),
    ] {
        let (c_size, rust_size) =
            unsafe { libs.functions::<StateBytes>(&format!("{prefix}_statebytes")) };
        let state_len = unsafe { c_size() };
        assert_eq!(state_len, unsafe { rust_size() });
        let (c_init, rust_init) = unsafe { libs.functions::<StateInit>(&format!("{prefix}_init")) };
        let (c_update, rust_update) =
            unsafe { libs.functions::<StateUpdate>(&format!("{prefix}_update")) };
        let (c_final, rust_final) =
            unsafe { libs.functions::<StateFinal>(&format!("{prefix}_final")) };
        let message = input(0x39e4, 513);
        for splits in [[0, 0, 513], [1, 31, 481], [64, 128, 321]] {
            let mut c_state = vec![0u8; state_len];
            let mut rust_state = vec![0u8; state_len];
            assert_eq!(unsafe { c_init(c_state.as_mut_ptr()) }, unsafe {
                rust_init(rust_state.as_mut_ptr())
            });
            let mut offset = 0;
            for chunk in splits {
                assert_eq!(
                    unsafe {
                        c_update(
                            c_state.as_mut_ptr(),
                            message[offset..].as_ptr(),
                            chunk as u64,
                        )
                    },
                    unsafe {
                        rust_update(
                            rust_state.as_mut_ptr(),
                            message[offset..].as_ptr(),
                            chunk as u64,
                        )
                    },
                    "{prefix}, chunk={chunk}"
                );
                offset += chunk;
            }
            let mut c_out = vec![0u8; out_len];
            let mut rust_out = vec![0u8; out_len];
            assert_eq!(
                unsafe { c_final(c_state.as_mut_ptr(), c_out.as_mut_ptr()) },
                unsafe { rust_final(rust_state.as_mut_ptr(), rust_out.as_mut_ptr()) }
            );
            assert_eq!(c_out, rust_out, "{prefix}");
        }
    }
}

#[test]
fn randomized_mac_secretbox_and_aead_paths_match() {
    let libs = unsafe { Libraries::load() };
    unsafe { initialize(&libs) };

    type Mac = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
    for (name, out_len, key_len) in [
        ("crypto_auth", 32, 32),
        ("crypto_auth_hmacsha256", 32, 32),
        ("crypto_auth_hmacsha512", 64, 32),
        ("crypto_auth_hmacsha512256", 32, 32),
        ("crypto_onetimeauth", 16, 32),
        ("crypto_shorthash", 8, 16),
        ("crypto_shorthash_siphashx24", 16, 16),
    ] {
        let (c, rust) = unsafe { libs.functions::<Mac>(name) };
        let key = input(0xabc0 + key_len as u64, key_len);
        for (case, len) in lengths().into_iter().enumerate() {
            let message = input(0xabc1 + case as u64, len);
            let mut c_out = vec![0u8; out_len];
            let mut rust_out = vec![0u8; out_len];
            assert_eq!(
                unsafe {
                    c(
                        c_out.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                    )
                },
                unsafe {
                    rust(
                        rust_out.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                    )
                },
                "{name}, len={len}"
            );
            assert_eq!(c_out, rust_out, "{name}, len={len}");
        }
    }

    type Secretbox = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
    let (c_seal, rust_seal) = unsafe { libs.functions::<Secretbox>("crypto_secretbox_easy") };
    let (c_open, rust_open) = unsafe { libs.functions::<Secretbox>("crypto_secretbox_open_easy") };
    let key = input(0x501, 32);
    let nonce = input(0x502, 24);
    for (case, len) in lengths().into_iter().enumerate() {
        let message = input(0x503 + case as u64, len);
        let mut c_box = vec![0u8; len + 16];
        let mut rust_box = vec![0u8; len + 16];
        assert_eq!(
            unsafe {
                c_seal(
                    c_box.as_mut_ptr(),
                    message.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                )
            },
            unsafe {
                rust_seal(
                    rust_box.as_mut_ptr(),
                    message.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                )
            }
        );
        assert_eq!(c_box, rust_box, "secretbox len={len}");
        let mut c_message = vec![0u8; len];
        let mut rust_message = vec![0u8; len];
        assert_eq!(
            unsafe {
                c_open(
                    c_message.as_mut_ptr(),
                    c_box.as_ptr(),
                    c_box.len() as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                )
            },
            unsafe {
                rust_open(
                    rust_message.as_mut_ptr(),
                    rust_box.as_ptr(),
                    rust_box.len() as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                )
            }
        );
        assert_eq!(c_message, rust_message);
    }

    type Aead = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *const u8,
        u64,
        *const u8,
        u64,
        *const u8,
        *const u8,
        *const u8,
    ) -> i32;
    for (name, nonce_len, overhead) in [
        ("crypto_aead_chacha20poly1305_encrypt", 8, 16),
        ("crypto_aead_chacha20poly1305_ietf_encrypt", 12, 16),
        ("crypto_aead_xchacha20poly1305_ietf_encrypt", 24, 16),
        ("crypto_aead_aegis128l_encrypt", 16, 32),
        ("crypto_aead_aegis256_encrypt", 32, 32),
    ] {
        let (c, rust) = unsafe { libs.functions::<Aead>(name) };
        let key = input(0xd00d, 32);
        let nonce = input(0xd00e, nonce_len);
        for (case, len) in lengths().into_iter().enumerate() {
            let message = input(0xd100 + case as u64, len);
            for ad_len in [0, 1, 16, 65] {
                let ad = input(0xd200 + ad_len as u64, ad_len);
                let mut c_out = vec![0u8; len + overhead];
                let mut rust_out = vec![0u8; len + overhead];
                let (mut c_len, mut rust_len) = (u64::MAX, u64::MAX);
                assert_eq!(
                    unsafe {
                        c(
                            c_out.as_mut_ptr(),
                            &mut c_len,
                            message.as_ptr(),
                            len as u64,
                            ad.as_ptr(),
                            ad_len as u64,
                            std::ptr::null(),
                            nonce.as_ptr(),
                            key.as_ptr(),
                        )
                    },
                    unsafe {
                        rust(
                            rust_out.as_mut_ptr(),
                            &mut rust_len,
                            message.as_ptr(),
                            len as u64,
                            ad.as_ptr(),
                            ad_len as u64,
                            std::ptr::null(),
                            nonce.as_ptr(),
                            key.as_ptr(),
                        )
                    },
                    "{name}, message={len}, ad={ad_len}"
                );
                assert_eq!(c_len, rust_len);
                assert_eq!(c_out, rust_out);
            }
        }
    }
}

#[test]
fn deterministic_public_key_kdf_and_error_paths_match() {
    let libs = unsafe { Libraries::load() };
    unsafe { initialize(&libs) };

    type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    type BoxFn =
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
    let (c_kp, rust_kp) = unsafe { libs.functions::<SeedKeypair>("crypto_box_seed_keypair") };
    let seed = input(0x7101, 32);
    let (mut c_pk, mut rust_pk) = (vec![0u8; 32], vec![0u8; 32]);
    let (mut c_sk, mut rust_sk) = (vec![0u8; 32], vec![0u8; 32]);
    assert_eq!(
        unsafe { c_kp(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr()) },
        unsafe { rust_kp(rust_pk.as_mut_ptr(), rust_sk.as_mut_ptr(), seed.as_ptr()) }
    );
    assert_eq!(
        (c_pk.as_slice(), c_sk.as_slice()),
        (rust_pk.as_slice(), rust_sk.as_slice())
    );
    let (c_box, rust_box) = unsafe { libs.functions::<BoxFn>("crypto_box_easy") };
    let nonce = input(0x7102, 24);
    for (case, len) in lengths().into_iter().enumerate() {
        let message = input(0x7103 + case as u64, len);
        let mut c_out = vec![0u8; len + 16];
        let mut rust_out = vec![0u8; len + 16];
        assert_eq!(
            unsafe {
                c_box(
                    c_out.as_mut_ptr(),
                    message.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    c_pk.as_ptr(),
                    c_sk.as_ptr(),
                )
            },
            unsafe {
                rust_box(
                    rust_out.as_mut_ptr(),
                    message.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    rust_pk.as_ptr(),
                    rust_sk.as_ptr(),
                )
            }
        );
        assert_eq!(c_out, rust_out);
    }

    let (c_sign_kp, rust_sign_kp) =
        unsafe { libs.functions::<SeedKeypair>("crypto_sign_seed_keypair") };
    let (mut c_sign_pk, mut rust_sign_pk) = (vec![0u8; 32], vec![0u8; 32]);
    let (mut c_sign_sk, mut rust_sign_sk) = (vec![0u8; 64], vec![0u8; 64]);
    assert_eq!(
        unsafe {
            c_sign_kp(
                c_sign_pk.as_mut_ptr(),
                c_sign_sk.as_mut_ptr(),
                seed.as_ptr(),
            )
        },
        unsafe {
            rust_sign_kp(
                rust_sign_pk.as_mut_ptr(),
                rust_sign_sk.as_mut_ptr(),
                seed.as_ptr(),
            )
        }
    );
    assert_eq!(c_sign_pk, rust_sign_pk);
    assert_eq!(c_sign_sk, rust_sign_sk);

    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    let (c_sign, rust_sign) = unsafe { libs.functions::<Sign>("crypto_sign_detached") };
    for (case, len) in lengths().into_iter().enumerate() {
        let message = input(0x7200 + case as u64, len);
        let (mut c_sig, mut rust_sig) = (vec![0u8; 64], vec![0u8; 64]);
        let (mut c_len, mut rust_len) = (0, 0);
        assert_eq!(
            unsafe {
                c_sign(
                    c_sig.as_mut_ptr(),
                    &mut c_len,
                    message.as_ptr(),
                    len as u64,
                    c_sign_sk.as_ptr(),
                )
            },
            unsafe {
                rust_sign(
                    rust_sig.as_mut_ptr(),
                    &mut rust_len,
                    message.as_ptr(),
                    len as u64,
                    rust_sign_sk.as_ptr(),
                )
            }
        );
        assert_eq!((c_sig, c_len), (rust_sig, rust_len));
    }

    type Kdf = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> i32;
    let (c_kdf, rust_kdf) = unsafe { libs.functions::<Kdf>("crypto_kdf_derive_from_key") };
    let context = *b"ctx-test";
    let key = input(0x7300, 32);
    for out_len in [16, 17, 31, 32, 63, 64] {
        let (mut c_out, mut rust_out) = (vec![0u8; out_len], vec![0u8; out_len]);
        assert_eq!(
            unsafe {
                c_kdf(
                    c_out.as_mut_ptr(),
                    out_len,
                    0x1234,
                    context.as_ptr().cast(),
                    key.as_ptr(),
                )
            },
            unsafe {
                rust_kdf(
                    rust_out.as_mut_ptr(),
                    out_len,
                    0x1234,
                    context.as_ptr().cast(),
                    key.as_ptr(),
                )
            }
        );
        assert_eq!(c_out, rust_out);
    }
    for out_len in [0, 15, 65, usize::MAX] {
        let (mut c_out, mut rust_out) = ([0u8; 1], [0u8; 1]);
        assert_eq!(
            unsafe {
                c_kdf(
                    c_out.as_mut_ptr(),
                    out_len,
                    0,
                    context.as_ptr().cast(),
                    key.as_ptr(),
                )
            },
            unsafe {
                rust_kdf(
                    rust_out.as_mut_ptr(),
                    out_len,
                    0,
                    context.as_ptr().cast(),
                    key.as_ptr(),
                )
            },
            "invalid KDF length {out_len}"
        );
    }
}

#[test]
fn codecs_padding_and_exact_error_sentinels_match() {
    let libs = unsafe { Libraries::load() };
    unsafe { initialize(&libs) };

    type Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
    type Unhex = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> i32;
    let (c_hex, rust_hex) = unsafe { libs.functions::<Hex>("sodium_bin2hex") };
    let (c_unhex, rust_unhex) = unsafe { libs.functions::<Unhex>("sodium_hex2bin") };
    for (case, len) in lengths().into_iter().enumerate() {
        let bytes = input(0x8100 + case as u64, len);
        let (mut c_text, mut rust_text) = (vec![0i8; len * 2 + 1], vec![0i8; len * 2 + 1]);
        unsafe {
            c_hex(c_text.as_mut_ptr(), c_text.len(), bytes.as_ptr(), len);
            rust_hex(rust_text.as_mut_ptr(), rust_text.len(), bytes.as_ptr(), len);
        }
        assert_eq!(c_text, rust_text);
        let (mut c_back, mut rust_back) = (vec![0u8; len], vec![0u8; len]);
        let (mut c_len, mut rust_len) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                c_unhex(
                    c_back.as_mut_ptr(),
                    len,
                    c_text.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut c_len,
                    std::ptr::null_mut(),
                )
            },
            unsafe {
                rust_unhex(
                    rust_back.as_mut_ptr(),
                    len,
                    rust_text.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut rust_len,
                    std::ptr::null_mut(),
                )
            }
        );
        assert_eq!((c_back, c_len), (rust_back, rust_len));
    }
    let invalid = b"0g";
    let (mut c_out, mut rust_out) = ([0u8; 1], [0u8; 1]);
    assert_eq!(
        unsafe {
            c_unhex(
                c_out.as_mut_ptr(),
                1,
                invalid.as_ptr().cast(),
                invalid.len(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        unsafe {
            rust_unhex(
                rust_out.as_mut_ptr(),
                1,
                invalid.as_ptr().cast(),
                invalid.len(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    );

    type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> i32;
    type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    let (c_ip2bin, rust_ip2bin) = unsafe { libs.functions::<Ip2Bin>("sodium_ip2bin") };
    let (c_bin2ip, rust_bin2ip) = unsafe { libs.functions::<Bin2Ip>("sodium_bin2ip") };
    for text in [
        "127.0.0.1",
        "::1",
        "2001:db8::1",
        "::ffff:192.0.2.1",
        "fe80::1%eth0",
    ] {
        let (mut c_bin, mut rust_bin) = ([0u8; 16], [0u8; 16]);
        assert_eq!(
            unsafe { c_ip2bin(c_bin.as_mut_ptr(), text.as_ptr().cast(), text.len()) },
            unsafe { rust_ip2bin(rust_bin.as_mut_ptr(), text.as_ptr().cast(), text.len()) },
            "{text}"
        );
        assert_eq!(c_bin, rust_bin, "{text}");
        let (mut c_text, mut rust_text) = ([0i8; 64], [0i8; 64]);
        let c_result = unsafe { c_bin2ip(c_text.as_mut_ptr(), c_text.len(), c_bin.as_ptr()) };
        let rust_result =
            unsafe { rust_bin2ip(rust_text.as_mut_ptr(), rust_text.len(), rust_bin.as_ptr()) };
        assert_eq!(c_result.is_null(), rust_result.is_null(), "{text}");
        assert_eq!(c_text, rust_text, "{text}");
    }
    for text in ["", "256.0.0.1", "1.2.3", ":::", "fe80::1%", "1.2.3.4%eth0"] {
        let (mut c_bin, mut rust_bin) = ([0u8; 16], [0u8; 16]);
        assert_eq!(
            unsafe { c_ip2bin(c_bin.as_mut_ptr(), text.as_ptr().cast(), text.len()) },
            unsafe { rust_ip2bin(rust_bin.as_mut_ptr(), text.as_ptr().cast(), text.len()) },
            "{text:?}"
        );
    }

    type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
    type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;
    let (c_pad, rust_pad) = unsafe { libs.functions::<Pad>("sodium_pad") };
    let (c_unpad, rust_unpad) = unsafe { libs.functions::<Unpad>("sodium_unpad") };
    for block in [1usize, 2, 16, 255] {
        for len in [0, 1, block.saturating_sub(1), block, block + 1, block * 3] {
            let max = len + block;
            let (mut c_buf, mut rust_buf) = (input(0x9100, max), input(0x9100, max));
            let (mut c_len, mut rust_len) = (0, 0);
            assert_eq!(
                unsafe { c_pad(&mut c_len, c_buf.as_mut_ptr(), len, block, max) },
                unsafe { rust_pad(&mut rust_len, rust_buf.as_mut_ptr(), len, block, max) }
            );
            assert_eq!((c_buf.as_slice(), c_len), (rust_buf.as_slice(), rust_len));
            let (mut c_plain, mut rust_plain) = (0, 0);
            assert_eq!(
                unsafe { c_unpad(&mut c_plain, c_buf.as_ptr(), c_len, block) },
                unsafe { rust_unpad(&mut rust_plain, rust_buf.as_ptr(), rust_len, block) }
            );
            assert_eq!(c_plain, rust_plain);
        }
    }

    type Open = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
    let (c_open, rust_open) = unsafe { libs.functions::<Open>("crypto_secretbox_open_easy") };
    let short = [0u8; 15];
    let nonce = [0u8; 24];
    let key = [0u8; 32];
    assert_eq!(
        unsafe {
            c_open(
                std::ptr::null_mut(),
                short.as_ptr(),
                short.len() as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        },
        unsafe {
            rust_open(
                std::ptr::null_mut(),
                short.as_ptr(),
                short.len() as u64,
                nonce.as_ptr(),
                key.as_ptr(),
            )
        }
    );
}

#[test]
fn misuse_probe() {
    let Ok(which) = std::env::var("SODIUM_MISUSE_PROBE") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = if which == "c" {
        root.join(C_SO)
    } else {
        root.join(RUST_SO)
    };
    unsafe {
        let lib = Library::new(path).unwrap();
        type InvalidVariant = unsafe extern "C" fn(usize, i32) -> usize;
        let function = *lib
            .get::<InvalidVariant>(b"sodium_base64_encoded_len")
            .unwrap();
        function(1, 2);
    }
}

#[cfg(unix)]
#[test]
fn out_of_range_enum_misuse_terminates_identically() {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().unwrap();
    let run = |library: &str| {
        Command::new(&exe)
            .arg("--exact")
            .arg("misuse_probe")
            .arg("--nocapture")
            .env("SODIUM_MISUSE_PROBE", library)
            .status()
            .unwrap()
    };
    let c = run("c");
    let rust = run("rust");
    assert!(!c.success());
    assert_eq!(c.signal(), rust.signal());
    assert_eq!(c.code(), rust.code());
}

#[test]
fn public_zero_boundary_probe() {
    let Ok(which) = std::env::var("SODIUM_ZERO_PROBE_LIBRARY") else {
        return;
    };
    let symbol = std::env::var("SODIUM_ZERO_PROBE_SYMBOL").unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = if which == "c" {
        root.join(C_SO)
    } else {
        root.join(RUST_SO)
    };
    unsafe {
        let lib = Library::new(path).unwrap();
        type ZeroCall = unsafe extern "C" fn(
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) -> usize;
        let function = *lib.get::<ZeroCall>(symbol.as_bytes()).unwrap();
        std::hint::black_box(function(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0));
    }
}

#[test]
fn every_public_entry_point_matches_for_zero_and_null_boundaries() {
    let exe = std::env::current_exe().unwrap();
    let configs = include_str!("../CONFIGS.md");
    let symbols: Vec<_> = configs
        .lines()
        .filter(|line| line.contains("| public header;"))
        .map(|line| line.split('`').nth(1).unwrap())
        .collect();
    assert_eq!(symbols.len(), 752);

    let run = |library: &str, symbol: &str| {
        Command::new(&exe)
            .arg("--exact")
            .arg("public_zero_boundary_probe")
            .env("SODIUM_ZERO_PROBE_LIBRARY", library)
            .env("SODIUM_ZERO_PROBE_SYMBOL", symbol)
            .output()
            .unwrap()
            .status
    };
    for symbol in symbols {
        let c = run("c", symbol);
        let rust = run("rust", symbol);
        assert_eq!(c.code(), rust.code(), "exit code mismatch for {symbol}");
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(c.signal(), rust.signal(), "signal mismatch for {symbol}");
        }
    }
}
