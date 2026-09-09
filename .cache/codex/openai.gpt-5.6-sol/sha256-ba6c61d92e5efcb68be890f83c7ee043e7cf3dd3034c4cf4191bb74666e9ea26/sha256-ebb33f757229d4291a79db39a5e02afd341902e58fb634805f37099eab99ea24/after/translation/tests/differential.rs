use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct Libraries {
    c: Library,
    rust: Library,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    crate_root()
        .parent()
        .unwrap()
        .join("c_src/build/libsodium.so")
}

fn rust_so() -> PathBuf {
    std::env::var_os("RUST_SODIUM_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target/release/liblibsodium.so"))
}

unsafe fn sym<'a, T>(library: &'a Library, name: &str) -> Symbol<'a, T> {
    unsafe { library.get(name.as_bytes()) }
        .unwrap_or_else(|error| panic!("failed to load {name}: {error}"))
}

fn with_libraries(test: impl FnOnce(&Libraries)) {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(
        c_so().is_file(),
        "missing C shared library: {}",
        c_so().display()
    );
    assert!(
        rust_so().is_file(),
        "missing Rust shared library: {}",
        rust_so().display()
    );
    let libraries = unsafe {
        Libraries {
            c: Library::new(c_so()).expect("load C shared library"),
            rust: Library::new(rust_so()).expect("load Rust shared library"),
        }
    };
    type Init = unsafe extern "C" fn() -> c_int;
    unsafe {
        assert_eq!(sym::<Init>(&libraries.c, "sodium_init")(), 0);
        assert_eq!(sym::<Init>(&libraries.rust, "sodium_init")(), 0);
    }
    test(&libraries);
}

fn nm_defined(path: &Path) -> Vec<(String, char, usize)> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only", "-P"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.to_owned();
            let kind = fields.next()?.chars().next()?;
            fields.next()?;
            let size = usize::from_str_radix(fields.next()?, 16).ok()?;
            Some((name, kind, size))
        })
        .collect()
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

#[test]
fn complete_dynamic_symbol_surface_resolves() {
    with_libraries(|libraries| {
        let c_symbols = nm_defined(&c_so());
        let rust_symbols = nm_defined(&rust_so());
        assert_eq!(c_symbols.len(), 890);
        assert_eq!(
            c_symbols.iter().map(|entry| &entry.0).collect::<Vec<_>>(),
            rust_symbols
                .iter()
                .map(|entry| &entry.0)
                .collect::<Vec<_>>()
        );

        for (name, kind, _) in c_symbols {
            unsafe {
                match kind {
                    'D' | 'B' | 'R' => {
                        let _ = sym::<*mut c_void>(&libraries.c, &name);
                        let _ = sym::<*mut c_void>(&libraries.rust, &name);
                    }
                    _ => {
                        let _ = sym::<unsafe extern "C" fn()>(&libraries.c, &name);
                        let _ = sym::<unsafe extern "C" fn()>(&libraries.rust, &name);
                    }
                }
            }
        }
    });
}

fn public_void_declarations() -> Vec<(String, String)> {
    let include = crate_root()
        .parent()
        .unwrap()
        .join("c_src/libsodium/include/sodium");
    let mut declarations = Vec::new();
    for entry in fs::read_dir(include).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("h") {
            continue;
        }
        let text = fs::read_to_string(path).unwrap();
        for statement in text.split(';') {
            let normalized = statement.split_whitespace().collect::<Vec<_>>().join(" ");
            if !normalized.contains("SODIUM_EXPORT") || !normalized.contains("(void)") {
                continue;
            }
            let prefix = normalized.split("(void)").next().unwrap().trim();
            let raw_name = prefix.split_whitespace().last().unwrap();
            let name = raw_name.trim_start_matches('*').to_owned();
            if name.contains('(') || name.is_empty() {
                continue;
            }
            let return_type = prefix[..prefix.len() - raw_name.len()].trim().to_owned();
            declarations.push((name, return_type));
        }
    }
    declarations.sort();
    declarations.dedup();
    declarations
}

#[test]
fn all_public_no_argument_queries_match() {
    with_libraries(|libraries| {
        let declarations = public_void_declarations();
        assert!(
            declarations.len() > 250,
            "only {} declarations",
            declarations.len()
        );
        let mut called = 0usize;

        for (name, return_type) in declarations {
            if name.contains("randombytes_random") || name.ends_with("_random") {
                continue;
            }
            unsafe {
                if return_type.contains("size_t") {
                    type F = unsafe extern "C" fn() -> usize;
                    assert_eq!(
                        sym::<F>(&libraries.c, &name)(),
                        sym::<F>(&libraries.rust, &name)(),
                        "{name}"
                    );
                } else if return_type.contains("unsigned long long") {
                    type F = unsafe extern "C" fn() -> u64;
                    assert_eq!(
                        sym::<F>(&libraries.c, &name)(),
                        sym::<F>(&libraries.rust, &name)(),
                        "{name}"
                    );
                } else if return_type.ends_with(" int") || return_type == "SODIUM_EXPORT int" {
                    type F = unsafe extern "C" fn() -> c_int;
                    assert_eq!(
                        sym::<F>(&libraries.c, &name)(),
                        sym::<F>(&libraries.rust, &name)(),
                        "{name}"
                    );
                } else if return_type.contains("const char *")
                    || return_type.contains("const char*")
                {
                    type F = unsafe extern "C" fn() -> *const c_char;
                    let c = sym::<F>(&libraries.c, &name)();
                    let rust = sym::<F>(&libraries.rust, &name)();
                    assert!(!c.is_null() && !rust.is_null(), "{name}");
                    assert_eq!(
                        CStr::from_ptr(c).to_bytes(),
                        CStr::from_ptr(rust).to_bytes(),
                        "{name}"
                    );
                } else {
                    continue;
                }
            }
            called += 1;
        }
        assert!(called > 250, "only called {called} query functions");
    });
}

#[test]
fn randomized_hash_mac_and_verify_outputs_match() {
    with_libraries(|libraries| unsafe {
        type Size = unsafe extern "C" fn() -> usize;
        type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
        type Mac = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
        type Verify = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

        let hashes = [
            "crypto_hash",
            "crypto_hash_sha256",
            "crypto_hash_sha3256",
            "crypto_hash_sha3512",
            "crypto_hash_sha512",
        ];
        let macs = [
            ("crypto_auth", "crypto_auth_bytes", "crypto_auth_keybytes"),
            (
                "crypto_auth_hmacsha256",
                "crypto_auth_hmacsha256_bytes",
                "crypto_auth_hmacsha256_keybytes",
            ),
            (
                "crypto_auth_hmacsha512",
                "crypto_auth_hmacsha512_bytes",
                "crypto_auth_hmacsha512_keybytes",
            ),
            (
                "crypto_auth_hmacsha512256",
                "crypto_auth_hmacsha512256_bytes",
                "crypto_auth_hmacsha512256_keybytes",
            ),
            (
                "crypto_onetimeauth",
                "crypto_onetimeauth_bytes",
                "crypto_onetimeauth_keybytes",
            ),
            (
                "crypto_onetimeauth_poly1305",
                "crypto_onetimeauth_poly1305_bytes",
                "crypto_onetimeauth_poly1305_keybytes",
            ),
            (
                "crypto_shorthash",
                "crypto_shorthash_bytes",
                "crypto_shorthash_keybytes",
            ),
            (
                "crypto_shorthash_siphash24",
                "crypto_shorthash_siphash24_bytes",
                "crypto_shorthash_siphash24_keybytes",
            ),
            (
                "crypto_shorthash_siphashx24",
                "crypto_shorthash_siphashx24_bytes",
                "crypto_shorthash_siphashx24_keybytes",
            ),
        ];
        let mut rng = Rng::new();

        for len in [0, 1, 2, 15, 16, 31, 32, 63, 64, 127, 255, 1024] {
            let input = rng.bytes(len);
            for name in hashes {
                let bytes_name = format!("{name}_bytes");
                let output_len = sym::<Size>(&libraries.c, &bytes_name)();
                assert_eq!(output_len, sym::<Size>(&libraries.rust, &bytes_name)());
                let mut c_out = vec![0xa5; output_len];
                let mut rust_out = vec![0x5a; output_len];
                assert_eq!(
                    sym::<Hash>(&libraries.c, name)(c_out.as_mut_ptr(), input.as_ptr(), len as u64),
                    sym::<Hash>(&libraries.rust, name)(
                        rust_out.as_mut_ptr(),
                        input.as_ptr(),
                        len as u64
                    ),
                    "{name}"
                );
                assert_eq!(c_out, rust_out, "{name}, len={len}");
            }

            for (name, bytes_name, keybytes_name) in macs {
                let output_len = sym::<Size>(&libraries.c, bytes_name)();
                let key_len = sym::<Size>(&libraries.c, keybytes_name)();
                let key = rng.bytes(key_len);
                let mut c_out = vec![0; output_len];
                let mut rust_out = vec![0; output_len];
                assert_eq!(
                    sym::<Mac>(&libraries.c, name)(
                        c_out.as_mut_ptr(),
                        input.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                    ),
                    sym::<Mac>(&libraries.rust, name)(
                        rust_out.as_mut_ptr(),
                        input.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                    ),
                    "{name}"
                );
                assert_eq!(c_out, rust_out, "{name}, len={len}");
            }
        }

        for width in [16usize, 32, 64] {
            let name = format!("crypto_verify_{width}");
            let mut left = rng.bytes(width);
            let right = left.clone();
            assert_eq!(
                sym::<Verify>(&libraries.c, &name)(left.as_ptr(), right.as_ptr()),
                0
            );
            assert_eq!(
                sym::<Verify>(&libraries.rust, &name)(left.as_ptr(), right.as_ptr()),
                0
            );
            left[width / 2] ^= 1;
            assert_eq!(
                sym::<Verify>(&libraries.c, &name)(left.as_ptr(), right.as_ptr()),
                sym::<Verify>(&libraries.rust, &name)(left.as_ptr(), right.as_ptr()),
                "{name} mismatch"
            );
        }
    });
}

#[test]
fn randomized_stream_and_core_outputs_match() {
    with_libraries(|libraries| unsafe {
        type Size = unsafe extern "C" fn() -> usize;
        type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
        type StreamXor =
            unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
        type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

        let streams = [
            "crypto_stream",
            "crypto_stream_chacha20",
            "crypto_stream_chacha20_ietf",
            "crypto_stream_salsa20",
            "crypto_stream_salsa2012",
            "crypto_stream_salsa208",
            "crypto_stream_xchacha20",
            "crypto_stream_xsalsa20",
        ];
        let cores = [
            "crypto_core_hchacha20",
            "crypto_core_hsalsa20",
            "crypto_core_salsa20",
            "crypto_core_salsa2012",
            "crypto_core_salsa208",
        ];
        let mut rng = Rng::new();

        for name in streams {
            let key_len = sym::<Size>(&libraries.c, &format!("{name}_keybytes"))();
            let nonce_len = sym::<Size>(&libraries.c, &format!("{name}_noncebytes"))();
            let key = rng.bytes(key_len);
            let nonce = rng.bytes(nonce_len);
            for len in [0usize, 1, 63, 64, 65, 255, 1024] {
                let message = rng.bytes(len);
                let mut c_stream = vec![0; len];
                let mut rust_stream = vec![0; len];
                assert_eq!(
                    sym::<Stream>(&libraries.c, name)(
                        c_stream.as_mut_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    sym::<Stream>(&libraries.rust, name)(
                        rust_stream.as_mut_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    "{name}"
                );
                assert_eq!(c_stream, rust_stream, "{name}, len={len}");

                let xor_name = format!("{name}_xor");
                let mut c_xor = vec![0; len];
                let mut rust_xor = vec![0; len];
                assert_eq!(
                    sym::<StreamXor>(&libraries.c, &xor_name)(
                        c_xor.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    sym::<StreamXor>(&libraries.rust, &xor_name)(
                        rust_xor.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    "{xor_name}"
                );
                assert_eq!(c_xor, rust_xor, "{xor_name}, len={len}");
            }
        }

        for name in cores {
            let output_len = sym::<Size>(&libraries.c, &format!("{name}_outputbytes"))();
            let input_len = sym::<Size>(&libraries.c, &format!("{name}_inputbytes"))();
            let key_len = sym::<Size>(&libraries.c, &format!("{name}_keybytes"))();
            let const_len = sym::<Size>(&libraries.c, &format!("{name}_constbytes"))();
            for constants_present in [false, true] {
                let input = rng.bytes(input_len);
                let key = rng.bytes(key_len);
                let constants = rng.bytes(const_len);
                let constants_ptr = if constants_present {
                    constants.as_ptr()
                } else {
                    std::ptr::null()
                };
                let mut c_out = vec![0; output_len];
                let mut rust_out = vec![0; output_len];
                assert_eq!(
                    sym::<Core>(&libraries.c, name)(
                        c_out.as_mut_ptr(),
                        input.as_ptr(),
                        key.as_ptr(),
                        constants_ptr,
                    ),
                    sym::<Core>(&libraries.rust, name)(
                        rust_out.as_mut_ptr(),
                        input.as_ptr(),
                        key.as_ptr(),
                        constants_ptr,
                    ),
                    "{name}"
                );
                assert_eq!(c_out, rust_out, "{name}, constants={constants_present}");
            }
        }
    });
}

#[test]
fn randomized_aead_round_trips_and_rejections_match() {
    with_libraries(|libraries| unsafe {
        type Size = unsafe extern "C" fn() -> usize;
        type Encrypt = unsafe extern "C" fn(
            *mut u8,
            *mut u64,
            *const u8,
            u64,
            *const u8,
            u64,
            *const u8,
            *const u8,
            *const u8,
        ) -> c_int;
        type Decrypt = unsafe extern "C" fn(
            *mut u8,
            *mut u64,
            *mut u8,
            *const u8,
            u64,
            *const u8,
            u64,
            *const u8,
            *const u8,
        ) -> c_int;

        let algorithms = [
            "crypto_aead_aegis128l",
            "crypto_aead_aegis256",
            "crypto_aead_aes256gcm",
            "crypto_aead_chacha20poly1305",
            "crypto_aead_chacha20poly1305_ietf",
            "crypto_aead_xchacha20poly1305_ietf",
        ];
        let mut rng = Rng::new();

        for name in algorithms {
            let key_len = sym::<Size>(&libraries.c, &format!("{name}_keybytes"))();
            let nonce_len = sym::<Size>(&libraries.c, &format!("{name}_npubbytes"))();
            let tag_len = sym::<Size>(&libraries.c, &format!("{name}_abytes"))();
            let key = rng.bytes(key_len);
            let nonce = rng.bytes(nonce_len);
            for (message_len, ad_len) in [(0, 0), (1, 0), (15, 1), (16, 17), (63, 31), (256, 64)] {
                let message = rng.bytes(message_len);
                let ad = rng.bytes(ad_len);
                let mut c_cipher = vec![0; message_len + tag_len];
                let mut rust_cipher = vec![0; message_len + tag_len];
                let mut c_len = 0u64;
                let mut rust_len = 0u64;
                let c_ret = sym::<Encrypt>(&libraries.c, &format!("{name}_encrypt"))(
                    c_cipher.as_mut_ptr(),
                    &mut c_len,
                    message.as_ptr(),
                    message_len as u64,
                    ad.as_ptr(),
                    ad_len as u64,
                    std::ptr::null(),
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
                let rust_ret = sym::<Encrypt>(&libraries.rust, &format!("{name}_encrypt"))(
                    rust_cipher.as_mut_ptr(),
                    &mut rust_len,
                    message.as_ptr(),
                    message_len as u64,
                    ad.as_ptr(),
                    ad_len as u64,
                    std::ptr::null(),
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(c_ret, rust_ret, "{name} encrypt");
                assert_eq!(c_len, rust_len, "{name} encrypt length");
                assert_eq!(
                    c_cipher, rust_cipher,
                    "{name}, m={message_len}, ad={ad_len}"
                );
                if c_ret != 0 {
                    continue;
                }

                let mut c_plain = vec![0; message_len];
                let mut rust_plain = vec![0; message_len];
                let mut c_plain_len = 0u64;
                let mut rust_plain_len = 0u64;
                assert_eq!(
                    sym::<Decrypt>(&libraries.c, &format!("{name}_decrypt"))(
                        c_plain.as_mut_ptr(),
                        &mut c_plain_len,
                        std::ptr::null_mut(),
                        c_cipher.as_ptr(),
                        c_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    sym::<Decrypt>(&libraries.rust, &format!("{name}_decrypt"))(
                        rust_plain.as_mut_ptr(),
                        &mut rust_plain_len,
                        std::ptr::null_mut(),
                        rust_cipher.as_ptr(),
                        rust_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    "{name} decrypt"
                );
                assert_eq!(c_plain_len, rust_plain_len);
                assert_eq!(c_plain, rust_plain);
                assert_eq!(c_plain, message);

                let mut tampered = c_cipher.clone();
                tampered[message_len] ^= 1;
                assert_eq!(
                    sym::<Decrypt>(&libraries.c, &format!("{name}_decrypt"))(
                        c_plain.as_mut_ptr(),
                        &mut c_plain_len,
                        std::ptr::null_mut(),
                        tampered.as_ptr(),
                        c_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    sym::<Decrypt>(&libraries.rust, &format!("{name}_decrypt"))(
                        rust_plain.as_mut_ptr(),
                        &mut rust_plain_len,
                        std::ptr::null_mut(),
                        tampered.as_ptr(),
                        rust_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    "{name} tampered rejection"
                );
            }
        }
    });
}

#[test]
fn codec_memory_and_boundary_results_match() {
    with_libraries(|libraries| unsafe {
        type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
        type Hex2Bin = unsafe extern "C" fn(
            *mut u8,
            usize,
            *const c_char,
            usize,
            *const c_char,
            *mut usize,
            *mut *const c_char,
        ) -> c_int;
        type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
        type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
        type IsZero = unsafe extern "C" fn(*const u8, usize) -> c_int;
        type Compare = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
        type Mut = unsafe extern "C" fn(*mut u8, usize);
        type Add = unsafe extern "C" fn(*mut u8, *const u8, usize);

        let mut rng = Rng::new();
        for len in [0usize, 1, 2, 15, 16, 31, 32, 255] {
            let input = rng.bytes(len);
            let mut c_hex = vec![0 as c_char; len * 2 + 1];
            let mut rust_hex = vec![0 as c_char; len * 2 + 1];
            sym::<Bin2Hex>(&libraries.c, "sodium_bin2hex")(
                c_hex.as_mut_ptr(),
                c_hex.len(),
                input.as_ptr(),
                len,
            );
            sym::<Bin2Hex>(&libraries.rust, "sodium_bin2hex")(
                rust_hex.as_mut_ptr(),
                rust_hex.len(),
                input.as_ptr(),
                len,
            );
            assert_eq!(c_hex, rust_hex);

            let mut c_back = vec![0; len];
            let mut rust_back = vec![0; len];
            let mut c_len = 0usize;
            let mut rust_len = 0usize;
            assert_eq!(
                sym::<Hex2Bin>(&libraries.c, "sodium_hex2bin")(
                    c_back.as_mut_ptr(),
                    c_back.len(),
                    c_hex.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut c_len,
                    std::ptr::null_mut(),
                ),
                sym::<Hex2Bin>(&libraries.rust, "sodium_hex2bin")(
                    rust_back.as_mut_ptr(),
                    rust_back.len(),
                    rust_hex.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut rust_len,
                    std::ptr::null_mut(),
                )
            );
            assert_eq!((c_len, c_back), (rust_len, rust_back));
        }

        let invalid = b"00xz";
        let mut c_out = [0u8; 8];
        let mut rust_out = [0u8; 8];
        let mut c_len = 0usize;
        let mut rust_len = 0usize;
        assert_eq!(
            sym::<Hex2Bin>(&libraries.c, "sodium_hex2bin")(
                c_out.as_mut_ptr(),
                c_out.len(),
                invalid.as_ptr().cast(),
                invalid.len(),
                std::ptr::null(),
                &mut c_len,
                std::ptr::null_mut(),
            ),
            sym::<Hex2Bin>(&libraries.rust, "sodium_hex2bin")(
                rust_out.as_mut_ptr(),
                rust_out.len(),
                invalid.as_ptr().cast(),
                invalid.len(),
                std::ptr::null(),
                &mut rust_len,
                std::ptr::null_mut(),
            )
        );
        assert_eq!((c_len, c_out), (rust_len, rust_out));

        for block in [0usize, 1, 2, 16, 255] {
            for len in [0usize, 1, 15, 16, 31] {
                let mut c_buf = vec![0u8; len + block.max(1)];
                let mut rust_buf = vec![0u8; len + block.max(1)];
                c_buf[..len].copy_from_slice(&rng.bytes(len));
                rust_buf[..len].copy_from_slice(&c_buf[..len]);
                let mut c_padded = 0usize;
                let mut rust_padded = 0usize;
                let c_ret = sym::<Pad>(&libraries.c, "sodium_pad")(
                    &mut c_padded,
                    c_buf.as_mut_ptr(),
                    len,
                    block,
                    c_buf.len(),
                );
                let rust_ret = sym::<Pad>(&libraries.rust, "sodium_pad")(
                    &mut rust_padded,
                    rust_buf.as_mut_ptr(),
                    len,
                    block,
                    rust_buf.len(),
                );
                assert_eq!(
                    (c_ret, c_padded, &c_buf),
                    (rust_ret, rust_padded, &rust_buf)
                );
                if c_ret == 0 {
                    let mut c_unpadded = 0usize;
                    let mut rust_unpadded = 0usize;
                    assert_eq!(
                        sym::<Unpad>(&libraries.c, "sodium_unpad")(
                            &mut c_unpadded,
                            c_buf.as_ptr(),
                            c_padded,
                            block,
                        ),
                        sym::<Unpad>(&libraries.rust, "sodium_unpad")(
                            &mut rust_unpadded,
                            rust_buf.as_ptr(),
                            rust_padded,
                            block,
                        )
                    );
                    assert_eq!(c_unpadded, rust_unpadded);
                }
            }
        }

        for len in [0usize, 1, 8, 32, 257] {
            let left = rng.bytes(len);
            let mut right = left.clone();
            assert_eq!(
                sym::<IsZero>(&libraries.c, "sodium_is_zero")(left.as_ptr(), len),
                sym::<IsZero>(&libraries.rust, "sodium_is_zero")(left.as_ptr(), len)
            );
            assert_eq!(
                sym::<Compare>(&libraries.c, "sodium_compare")(left.as_ptr(), right.as_ptr(), len),
                sym::<Compare>(&libraries.rust, "sodium_compare")(
                    left.as_ptr(),
                    right.as_ptr(),
                    len
                )
            );
            if len != 0 {
                right[len / 2] ^= 1;
            }
            assert_eq!(
                sym::<Compare>(&libraries.c, "sodium_compare")(left.as_ptr(), right.as_ptr(), len),
                sym::<Compare>(&libraries.rust, "sodium_compare")(
                    left.as_ptr(),
                    right.as_ptr(),
                    len
                )
            );
            let mut c_increment = left.clone();
            let mut rust_increment = left.clone();
            sym::<Mut>(&libraries.c, "sodium_increment")(c_increment.as_mut_ptr(), len);
            sym::<Mut>(&libraries.rust, "sodium_increment")(rust_increment.as_mut_ptr(), len);
            assert_eq!(c_increment, rust_increment);
            let addend = rng.bytes(len);
            sym::<Add>(&libraries.c, "sodium_add")(c_increment.as_mut_ptr(), addend.as_ptr(), len);
            sym::<Add>(&libraries.rust, "sodium_add")(
                rust_increment.as_mut_ptr(),
                addend.as_ptr(),
                len,
            );
            assert_eq!(c_increment, rust_increment);
        }
    });
}

#[test]
fn explicit_parameter_rejections_match() {
    with_libraries(|libraries| unsafe {
        type Size = unsafe extern "C" fn() -> usize;
        type GenericHash =
            unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
        type Derive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
        type ScalarMult = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

        let input = b"parameter boundary";
        let hash_min = sym::<Size>(&libraries.c, "crypto_generichash_bytes_min")();
        let hash_max = sym::<Size>(&libraries.c, "crypto_generichash_bytes_max")();
        let key_max = sym::<Size>(&libraries.c, "crypto_generichash_keybytes_max")();
        let mut output = vec![0u8; hash_max + 1];
        let oversized_key = vec![7u8; key_max + 1];
        for (out_len, key_len) in [
            (hash_min - 1, 0),
            (hash_max + 1, 0),
            (hash_min, key_max + 1),
        ] {
            let key_ptr = if key_len == 0 {
                std::ptr::null()
            } else {
                oversized_key.as_ptr()
            };
            assert_eq!(
                sym::<GenericHash>(&libraries.c, "crypto_generichash")(
                    output.as_mut_ptr(),
                    out_len,
                    input.as_ptr(),
                    input.len() as u64,
                    key_ptr,
                    key_len,
                ),
                sym::<GenericHash>(&libraries.rust, "crypto_generichash")(
                    output.as_mut_ptr(),
                    out_len,
                    input.as_ptr(),
                    input.len() as u64,
                    key_ptr,
                    key_len,
                ),
                "generichash out_len={out_len}, key_len={key_len}"
            );
        }

        let subkey_min = sym::<Size>(&libraries.c, "crypto_kdf_blake2b_bytes_min")();
        let subkey_max = sym::<Size>(&libraries.c, "crypto_kdf_blake2b_bytes_max")();
        let key_len = sym::<Size>(&libraries.c, "crypto_kdf_blake2b_keybytes")();
        let context_len = sym::<Size>(&libraries.c, "crypto_kdf_blake2b_contextbytes")();
        let key = vec![3u8; key_len];
        let context = vec![b'c' as c_char; context_len];
        let mut subkey = vec![0u8; subkey_max + 1];
        for subkey_len in [subkey_min - 1, subkey_max + 1] {
            assert_eq!(
                sym::<Derive>(&libraries.c, "crypto_kdf_blake2b_derive_from_key")(
                    subkey.as_mut_ptr(),
                    subkey_len,
                    1,
                    context.as_ptr(),
                    key.as_ptr(),
                ),
                sym::<Derive>(&libraries.rust, "crypto_kdf_blake2b_derive_from_key")(
                    subkey.as_mut_ptr(),
                    subkey_len,
                    1,
                    context.as_ptr(),
                    key.as_ptr(),
                ),
                "KDF subkey_len={subkey_len}"
            );
        }

        let scalar_len = sym::<Size>(&libraries.c, "crypto_scalarmult_curve25519_scalarbytes")();
        let point_len = sym::<Size>(&libraries.c, "crypto_scalarmult_curve25519_bytes")();
        let scalar = vec![0u8; scalar_len];
        let mut point = vec![0u8; point_len];
        point[0] = 9;
        let mut c_out = vec![0u8; point_len];
        let mut rust_out = vec![0u8; point_len];
        assert_eq!(
            sym::<ScalarMult>(&libraries.c, "crypto_scalarmult_curve25519")(
                c_out.as_mut_ptr(),
                scalar.as_ptr(),
                point.as_ptr(),
            ),
            sym::<ScalarMult>(&libraries.rust, "crypto_scalarmult_curve25519")(
                rust_out.as_mut_ptr(),
                scalar.as_ptr(),
                point.as_ptr(),
            )
        );
        assert_eq!(c_out, rust_out);
    });
}

fn run_abort_probe(so: &Path, case: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "abort_probe", "--nocapture"])
        .env("SODIUM_PROBE_SO", so)
        .env("SODIUM_PROBE_CASE", case)
        .output()
        .expect("run abort probe")
}

#[test]
fn abort_probe() {
    let Some(so) = std::env::var_os("SODIUM_PROBE_SO") else {
        return;
    };
    let case = std::env::var("SODIUM_PROBE_CASE").unwrap();
    let library = unsafe { Library::new(so).unwrap() };
    unsafe {
        match case.as_str() {
            "invalid_base64_variant" => {
                type F = unsafe extern "C" fn(usize, c_int) -> usize;
                let _ = sym::<F>(&library, "sodium_base64_encoded_len")(1, 0);
            }
            "oversized_bin2hex" => {
                type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
                let _ = sym::<F>(&library, "sodium_bin2hex")(
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                    usize::MAX,
                );
            }
            "null_hash_output" => {
                type F = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
                let _ = sym::<F>(&library, "crypto_hash_sha256")(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    0,
                );
            }
            _ => panic!("unknown abort probe case"),
        }
    }
}

#[test]
fn process_level_rejections_match() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for case in [
        "invalid_base64_variant",
        "oversized_bin2hex",
        "null_hash_output",
    ] {
        let c = run_abort_probe(&c_so(), case);
        let rust = run_abort_probe(&rust_so(), case);
        assert!(!c.status.success(), "C did not reject {case}");
        assert!(!rust.status.success(), "Rust did not reject {case}");
        assert_eq!(c.status.code(), rust.status.code(), "{case}");
        assert_eq!(c.status.signal(), rust.status.signal(), "{case}");
    }
}
