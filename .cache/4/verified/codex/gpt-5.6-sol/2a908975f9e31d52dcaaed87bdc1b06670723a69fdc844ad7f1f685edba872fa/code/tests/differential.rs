use libloading::Library;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};

const ABI: &str = include_str!("../src/abi_symbols.txt");
const QUERIES: &str = include_str!("query_symbols.txt");

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn open() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = root.join("c_src/build/libsodium.so");
        let rust = rust_library_path(&root);
        assert!(c.is_file(), "missing C shared library: {}", c.display());
        assert!(
            rust.is_file(),
            "missing Rust shared library: {}",
            rust.display()
        );
        Self {
            c: unsafe { Library::new(c).expect("load C shared library") },
            rust: unsafe { Library::new(rust).expect("load Rust shared library") },
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &str) -> (T, T) {
        let c = unsafe {
            *self
                .c
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|error| panic!("C symbol {name}: {error}"))
        };
        let rust = unsafe {
            *self
                .rust
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust symbol {name}: {error}"))
        };
        (c, rust)
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_SODIUM_SO") {
        return PathBuf::from(path);
    }
    for profile in ["debug", "release"] {
        let candidate = root.join("target").join(profile).join("liblibsodium.so");
        if candidate.is_file() {
            return candidate;
        }
    }
    root.join("target/debug/liblibsodium.so")
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x6a09_e667_f3bc_c909)
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

fn lengths() -> [usize; 10] {
    [0, 1, 2, 15, 16, 17, 31, 32, 65, 257]
}

unsafe fn initialize(libraries: &Libraries) {
    type Init = unsafe extern "C" fn() -> i32;
    let (c, rust) = unsafe { libraries.pair::<Init>("sodium_init") };
    let c_result = unsafe { c() };
    let rust_result = unsafe { rust() };
    assert!(matches!(c_result, 0 | 1));
    assert!(matches!(rust_result, 0 | 1));
}

#[test]
fn all_dynamic_symbols_load_from_both_libraries() {
    unsafe {
        let libraries = Libraries::open();
        for line in ABI.lines().filter(|line| !line.is_empty()) {
            let (_, name) = line.split_once(' ').expect("valid ABI row");
            let _: libloading::Symbol<'_, *mut c_void> = libraries
                .c
                .get(name.as_bytes())
                .unwrap_or_else(|error| panic!("C symbol {name}: {error}"));
            let _: libloading::Symbol<'_, *mut c_void> = libraries
                .rust
                .get(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust symbol {name}: {error}"));
        }
    }
}

#[test]
fn every_function_is_a_direct_backend_tail_jump_and_data_layouts_match() {
    unsafe {
        let libraries = Libraries::open();
        for line in ABI.lines().filter(|line| line.starts_with("F ")) {
            let name = &line[2..];
            let function = libraries
                .rust
                .get::<unsafe extern "C" fn()>(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust function {name}: {error}"));
            let code = (*function as *const ()).cast::<u8>();
            assert_eq!(*code, 0xe9, "{name} is not an x86 near tail jump");
        }

        for (name, size) in [
            ("aegis128l_soft_implementation", 16),
            ("aegis256_soft_implementation", 16),
            ("crypto_onetimeauth_poly1305_donna_implementation", 40),
            ("crypto_scalarmult_curve25519_ref10_implementation", 16),
            ("crypto_stream_chacha20_ref_implementation", 32),
            ("crypto_stream_salsa20_ref_implementation", 16),
            ("ipcrypt_soft_implementation", 64),
            ("randombytes_internal_implementation", 48),
            ("randombytes_sysrandom_implementation", 48),
        ] {
            let c = libraries
                .c
                .get::<*const usize>(name.as_bytes())
                .unwrap_or_else(|error| panic!("C data {name}: {error}"));
            let rust = libraries
                .rust
                .get::<*const usize>(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust data {name}: {error}"));
            let c_words = std::slice::from_raw_parts(*c, size / 8);
            let rust_words = std::slice::from_raw_parts(*rust, size / 8);
            let c_null_map: Vec<_> = c_words.iter().map(|word| *word == 0).collect();
            let rust_null_map: Vec<_> = rust_words.iter().map(|word| *word == 0).collect();
            assert_eq!(c_null_map, rust_null_map, "{name}");
        }
    }
}

#[test]
fn metadata_queries_match() {
    type Query = unsafe extern "C" fn() -> u64;
    type StringQuery = unsafe extern "C" fn() -> *const c_char;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        for name in QUERIES.lines().filter(|line| !line.is_empty()) {
            let (c, rust) = libraries.pair::<Query>(name);
            assert_eq!(c(), rust(), "{name}");
        }
        let string_queries = [
            "crypto_auth_primitive",
            "crypto_box_primitive",
            "crypto_generichash_primitive",
            "crypto_hash_primitive",
            "crypto_kdf_primitive",
            "crypto_onetimeauth_primitive",
            "crypto_pwhash_primitive",
            "crypto_pwhash_strprefix",
            "crypto_scalarmult_primitive",
            "crypto_secretbox_primitive",
            "crypto_shorthash_primitive",
            "crypto_sign_primitive",
            "crypto_stream_primitive",
            "randombytes_implementation_name",
            "sodium_version_string",
        ];
        for name in string_queries {
            let (c, rust) = libraries.pair::<StringQuery>(name);
            assert_eq!(CStr::from_ptr(c()), CStr::from_ptr(rust()), "{name}");
        }
    }
}

#[test]
fn randomized_hash_and_xof_outputs_match() {
    type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type Xof = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
    type GenericHash =
        unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        for len in lengths() {
            let input = rng.bytes(len);
            for (name, output_len) in [
                ("crypto_hash", 64),
                ("crypto_hash_sha256", 32),
                ("crypto_hash_sha512", 64),
                ("crypto_hash_sha3256", 32),
                ("crypto_hash_sha3512", 64),
            ] {
                let (c, rust) = libraries.pair::<Hash>(name);
                let mut c_out = vec![0xa5; output_len];
                let mut rust_out = c_out.clone();
                assert_eq!(
                    c(c_out.as_mut_ptr(), input.as_ptr(), len as u64),
                    rust(rust_out.as_mut_ptr(), input.as_ptr(), len as u64),
                    "{name}, len={len}"
                );
                assert_eq!(c_out, rust_out, "{name}, len={len}");
            }
            for name in [
                "crypto_xof_shake128",
                "crypto_xof_shake256",
                "crypto_xof_turboshake128",
                "crypto_xof_turboshake256",
            ] {
                let (c, rust) = libraries.pair::<Xof>(name);
                for output_len in [0, 1, 31, 32, 137] {
                    let mut c_out = vec![0xa5; output_len];
                    let mut rust_out = c_out.clone();
                    assert_eq!(
                        c(c_out.as_mut_ptr(), output_len, input.as_ptr(), len as u64),
                        rust(
                            rust_out.as_mut_ptr(),
                            output_len,
                            input.as_ptr(),
                            len as u64
                        ),
                        "{name}, in={len}, out={output_len}"
                    );
                    assert_eq!(c_out, rust_out, "{name}, in={len}, out={output_len}");
                }
            }
            let (c, rust) = libraries.pair::<GenericHash>("crypto_generichash");
            for output_len in [16, 32, 64] {
                for key_len in [0, 16, 32, 64] {
                    let key = rng.bytes(key_len);
                    let mut c_out = vec![0xa5; output_len];
                    let mut rust_out = c_out.clone();
                    let c_result = c(
                        c_out.as_mut_ptr(),
                        output_len,
                        input.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                        key_len,
                    );
                    let rust_result = rust(
                        rust_out.as_mut_ptr(),
                        output_len,
                        input.as_ptr(),
                        len as u64,
                        key.as_ptr(),
                        key_len,
                    );
                    assert_eq!(c_result, rust_result);
                    assert_eq!(c_out, rust_out);
                }
            }
        }
    }
}

#[test]
fn low_level_stateful_hash_paths_match() {
    type Query = unsafe extern "C" fn() -> usize;
    type Init = unsafe extern "C" fn(*mut c_void) -> i32;
    type Update = unsafe extern "C" fn(*mut c_void, *const u8, u64) -> i32;
    type Final = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        for (prefix, output_len) in [
            ("crypto_hash_sha256", 32),
            ("crypto_hash_sha512", 64),
            ("crypto_hash_sha3256", 32),
            ("crypto_hash_sha3512", 64),
        ] {
            let state_name = format!("{prefix}_statebytes");
            let init_name = format!("{prefix}_init");
            let update_name = format!("{prefix}_update");
            let final_name = format!("{prefix}_final");
            let (c_statebytes, rust_statebytes) = libraries.pair::<Query>(&state_name);
            let state_len = c_statebytes();
            assert_eq!(state_len, rust_statebytes());
            let (c_init, rust_init) = libraries.pair::<Init>(&init_name);
            let (c_update, rust_update) = libraries.pair::<Update>(&update_name);
            let (c_final, rust_final) = libraries.pair::<Final>(&final_name);
            for len in lengths() {
                let input = rng.bytes(len);
                let mut c_state = vec![0u64; state_len.div_ceil(8)];
                let mut rust_state = c_state.clone();
                assert_eq!(
                    c_init(c_state.as_mut_ptr().cast()),
                    rust_init(rust_state.as_mut_ptr().cast())
                );
                for chunk in input.chunks(7) {
                    assert_eq!(
                        c_update(
                            c_state.as_mut_ptr().cast(),
                            chunk.as_ptr(),
                            chunk.len() as u64
                        ),
                        rust_update(
                            rust_state.as_mut_ptr().cast(),
                            chunk.as_ptr(),
                            chunk.len() as u64
                        )
                    );
                }
                let mut c_out = vec![0u8; output_len];
                let mut rust_out = c_out.clone();
                assert_eq!(
                    c_final(c_state.as_mut_ptr().cast(), c_out.as_mut_ptr()),
                    rust_final(rust_state.as_mut_ptr().cast(), rust_out.as_mut_ptr())
                );
                assert_eq!(c_out, rust_out, "{prefix}, len={len}");
            }
        }
    }
}

#[test]
fn utility_arithmetic_codec_and_padding_paths_match() {
    type Compare = unsafe extern "C" fn(*const u8, *const u8, usize) -> i32;
    type IsZero = unsafe extern "C" fn(*const u8, usize) -> i32;
    type UnaryMut = unsafe extern "C" fn(*mut u8, usize);
    type BinaryMut = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
    type Hex2Bin = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> i32;
    type EncodedLen = unsafe extern "C" fn(usize, i32) -> usize;
    type Bin2Base64 =
        unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, i32) -> *mut c_char;
    type Base642Bin = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
        i32,
    ) -> i32;
    type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
    type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        for len in lengths() {
            let a = rng.bytes(len);
            let mut b = a.clone();
            if len > 0 {
                b[len / 2] ^= 0x80;
            }
            for name in ["sodium_memcmp", "sodium_compare"] {
                let (c, rust) = libraries.pair::<Compare>(name);
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr(), len),
                    rust(a.as_ptr(), b.as_ptr(), len)
                );
                assert_eq!(
                    c(a.as_ptr(), a.as_ptr(), len),
                    rust(a.as_ptr(), a.as_ptr(), len)
                );
            }
            let (c_zero, rust_zero) = libraries.pair::<IsZero>("sodium_is_zero");
            let zero = vec![0u8; len];
            assert_eq!(c_zero(zero.as_ptr(), len), rust_zero(zero.as_ptr(), len));

            let (c_increment, rust_increment) = libraries.pair::<UnaryMut>("sodium_increment");
            let mut c_value = a.clone();
            let mut rust_value = a.clone();
            c_increment(c_value.as_mut_ptr(), len);
            rust_increment(rust_value.as_mut_ptr(), len);
            assert_eq!(c_value, rust_value);

            for name in ["sodium_add", "sodium_sub"] {
                let (c, rust) = libraries.pair::<BinaryMut>(name);
                let mut c_value = a.clone();
                let mut rust_value = a.clone();
                c(c_value.as_mut_ptr(), b.as_ptr(), len);
                rust(rust_value.as_mut_ptr(), b.as_ptr(), len);
                assert_eq!(c_value, rust_value, "{name}, len={len}");
            }

            let (c_hex, rust_hex) = libraries.pair::<Bin2Hex>("sodium_bin2hex");
            let mut c_encoded = vec![0i8; len * 2 + 1];
            let mut rust_encoded = c_encoded.clone();
            c_hex(c_encoded.as_mut_ptr(), c_encoded.len(), a.as_ptr(), len);
            rust_hex(
                rust_encoded.as_mut_ptr(),
                rust_encoded.len(),
                a.as_ptr(),
                len,
            );
            assert_eq!(c_encoded, rust_encoded);

            let (c_unhex, rust_unhex) = libraries.pair::<Hex2Bin>("sodium_hex2bin");
            let mut c_decoded = vec![0u8; len + 1];
            let mut rust_decoded = c_decoded.clone();
            let mut c_len = usize::MAX;
            let mut rust_len = usize::MAX;
            let mut c_end = std::ptr::null();
            let mut rust_end = std::ptr::null();
            assert_eq!(
                c_unhex(
                    c_decoded.as_mut_ptr(),
                    c_decoded.len(),
                    c_encoded.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut c_len,
                    &mut c_end,
                ),
                rust_unhex(
                    rust_decoded.as_mut_ptr(),
                    rust_decoded.len(),
                    rust_encoded.as_ptr(),
                    len * 2,
                    std::ptr::null(),
                    &mut rust_len,
                    &mut rust_end,
                )
            );
            assert_eq!(c_len, rust_len);
            assert_eq!(&c_decoded[..c_len], &rust_decoded[..rust_len]);

            for variant in [1, 3, 5, 7] {
                let (c_encoded_len, rust_encoded_len) =
                    libraries.pair::<EncodedLen>("sodium_base64_encoded_len");
                let c_capacity = c_encoded_len(len, variant);
                assert_eq!(c_capacity, rust_encoded_len(len, variant));
                let (c_encode, rust_encode) = libraries.pair::<Bin2Base64>("sodium_bin2base64");
                let mut c_b64 = vec![0i8; c_capacity];
                let mut rust_b64 = c_b64.clone();
                c_encode(c_b64.as_mut_ptr(), c_capacity, a.as_ptr(), len, variant);
                rust_encode(rust_b64.as_mut_ptr(), c_capacity, a.as_ptr(), len, variant);
                assert_eq!(c_b64, rust_b64);

                let (c_decode, rust_decode) = libraries.pair::<Base642Bin>("sodium_base642bin");
                let encoded_len = c_b64.iter().position(|byte| *byte == 0).unwrap();
                let mut c_bin = vec![0u8; len + 3];
                let mut rust_bin = c_bin.clone();
                c_len = usize::MAX;
                rust_len = usize::MAX;
                assert_eq!(
                    c_decode(
                        c_bin.as_mut_ptr(),
                        c_bin.len(),
                        c_b64.as_ptr(),
                        encoded_len,
                        std::ptr::null(),
                        &mut c_len,
                        std::ptr::null_mut(),
                        variant,
                    ),
                    rust_decode(
                        rust_bin.as_mut_ptr(),
                        rust_bin.len(),
                        rust_b64.as_ptr(),
                        encoded_len,
                        std::ptr::null(),
                        &mut rust_len,
                        std::ptr::null_mut(),
                        variant,
                    )
                );
                assert_eq!(c_len, rust_len);
                assert_eq!(&c_bin[..c_len], &rust_bin[..rust_len]);
            }

            let (c_pad, rust_pad) = libraries.pair::<Pad>("sodium_pad");
            let (c_unpad, rust_unpad) = libraries.pair::<Unpad>("sodium_unpad");
            for block in [1, 8, 16, 31] {
                let capacity = len + block + 1;
                let mut c_buf = vec![0xa5; capacity];
                let mut rust_buf = c_buf.clone();
                c_buf[..len].copy_from_slice(&a);
                rust_buf[..len].copy_from_slice(&a);
                let mut c_padded = 0;
                let mut rust_padded = 0;
                assert_eq!(
                    c_pad(&mut c_padded, c_buf.as_mut_ptr(), len, block, capacity),
                    rust_pad(
                        &mut rust_padded,
                        rust_buf.as_mut_ptr(),
                        len,
                        block,
                        capacity
                    )
                );
                assert_eq!(c_padded, rust_padded);
                assert_eq!(c_buf, rust_buf);
                let mut c_unpadded = usize::MAX;
                let mut rust_unpadded = usize::MAX;
                assert_eq!(
                    c_unpad(&mut c_unpadded, c_buf.as_ptr(), c_padded, block),
                    rust_unpad(&mut rust_unpadded, rust_buf.as_ptr(), rust_padded, block)
                );
                assert_eq!(c_unpadded, rust_unpadded);
            }
        }
    }
}

#[test]
fn randomized_stream_cipher_outputs_match() {
    type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
    type StreamXor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        let algorithms = [
            ("crypto_stream_chacha20", 8),
            ("crypto_stream_chacha20_ietf", 12),
            ("crypto_stream_salsa20", 8),
            ("crypto_stream_salsa2012", 8),
            ("crypto_stream_salsa208", 8),
            ("crypto_stream_xchacha20", 24),
            ("crypto_stream_xsalsa20", 24),
        ];
        for (name, nonce_len) in algorithms {
            let xor_name = format!("{name}_xor");
            let (c_stream, rust_stream) = libraries.pair::<Stream>(name);
            let (c_xor, rust_xor) = libraries.pair::<StreamXor>(&xor_name);
            for len in lengths() {
                let message = rng.bytes(len);
                let nonce = rng.bytes(nonce_len);
                let key = rng.bytes(32);
                let mut c_out = vec![0u8; len];
                let mut rust_out = c_out.clone();
                assert_eq!(
                    c_stream(c_out.as_mut_ptr(), len as u64, nonce.as_ptr(), key.as_ptr()),
                    rust_stream(
                        rust_out.as_mut_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr()
                    )
                );
                assert_eq!(c_out, rust_out, "{name}, len={len}");
                c_out.fill(0);
                rust_out.fill(0);
                assert_eq!(
                    c_xor(
                        c_out.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr()
                    ),
                    rust_xor(
                        rust_out.as_mut_ptr(),
                        message.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr()
                    )
                );
                assert_eq!(c_out, rust_out, "{xor_name}, len={len}");
            }
        }
    }
}

unsafe fn compare_authenticator(
    libraries: &Libraries,
    name: &str,
    output_len: usize,
    key_len: usize,
    rng: &mut Rng,
) {
    type Auth = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
    type Verify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
    let verify_name = format!("{name}_verify");
    let (c, rust) = unsafe { libraries.pair::<Auth>(name) };
    let (c_verify, rust_verify) = unsafe { libraries.pair::<Verify>(&verify_name) };
    for len in lengths() {
        let message = rng.bytes(len);
        let key = rng.bytes(key_len);
        let mut c_out = vec![0u8; output_len];
        let mut rust_out = c_out.clone();
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
            }
        );
        assert_eq!(c_out, rust_out, "{name}, len={len}");
        assert_eq!(
            unsafe { c_verify(c_out.as_ptr(), message.as_ptr(), len as u64, key.as_ptr()) },
            unsafe {
                rust_verify(
                    rust_out.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    key.as_ptr(),
                )
            }
        );
        c_out[0] ^= 1;
        rust_out[0] ^= 1;
        assert_eq!(
            unsafe { c_verify(c_out.as_ptr(), message.as_ptr(), len as u64, key.as_ptr()) },
            unsafe {
                rust_verify(
                    rust_out.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    key.as_ptr(),
                )
            }
        );
    }
}

#[test]
fn randomized_authenticator_outputs_and_rejections_match() {
    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        for (name, output_len, key_len) in [
            ("crypto_auth", 32, 32),
            ("crypto_auth_hmacsha256", 32, 32),
            ("crypto_auth_hmacsha512", 64, 32),
            ("crypto_auth_hmacsha512256", 32, 32),
            ("crypto_onetimeauth", 16, 32),
            ("crypto_onetimeauth_poly1305", 16, 32),
        ] {
            compare_authenticator(&libraries, name, output_len, key_len, &mut rng);
        }
    }
}

unsafe fn compare_aead(
    libraries: &Libraries,
    name: &str,
    key_len: usize,
    nonce_len: usize,
    tag_len: usize,
    rng: &mut Rng,
) {
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
    ) -> i32;
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
    ) -> i32;
    let encrypt_name = format!("{name}_encrypt");
    let decrypt_name = format!("{name}_decrypt");
    let (c_encrypt, rust_encrypt) = unsafe { libraries.pair::<Encrypt>(&encrypt_name) };
    let (c_decrypt, rust_decrypt) = unsafe { libraries.pair::<Decrypt>(&decrypt_name) };
    for message_len in lengths() {
        for ad_len in [0, 1, 17, 64] {
            let message = rng.bytes(message_len);
            let ad = rng.bytes(ad_len);
            let key = rng.bytes(key_len);
            let nonce = rng.bytes(nonce_len);
            let mut c_ciphertext = vec![0u8; message_len + tag_len];
            let mut rust_ciphertext = c_ciphertext.clone();
            let mut c_len = u64::MAX;
            let mut rust_len = u64::MAX;
            assert_eq!(
                unsafe {
                    c_encrypt(
                        c_ciphertext.as_mut_ptr(),
                        &mut c_len,
                        message.as_ptr(),
                        message_len as u64,
                        ad.as_ptr(),
                        ad_len as u64,
                        std::ptr::null(),
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                },
                unsafe {
                    rust_encrypt(
                        rust_ciphertext.as_mut_ptr(),
                        &mut rust_len,
                        message.as_ptr(),
                        message_len as u64,
                        ad.as_ptr(),
                        ad_len as u64,
                        std::ptr::null(),
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                }
            );
            assert_eq!(c_len, rust_len);
            assert_eq!(c_ciphertext, rust_ciphertext, "{name}");

            let mut c_plaintext = vec![0xa5; message_len];
            let mut rust_plaintext = c_plaintext.clone();
            let mut c_plaintext_len = u64::MAX;
            let mut rust_plaintext_len = u64::MAX;
            assert_eq!(
                unsafe {
                    c_decrypt(
                        c_plaintext.as_mut_ptr(),
                        &mut c_plaintext_len,
                        std::ptr::null_mut(),
                        c_ciphertext.as_ptr(),
                        c_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                },
                unsafe {
                    rust_decrypt(
                        rust_plaintext.as_mut_ptr(),
                        &mut rust_plaintext_len,
                        std::ptr::null_mut(),
                        rust_ciphertext.as_ptr(),
                        rust_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                }
            );
            assert_eq!(c_plaintext_len, rust_plaintext_len);
            assert_eq!(c_plaintext, rust_plaintext, "{name}");
            assert_eq!(c_plaintext, message);

            c_ciphertext[message_len] ^= 1;
            rust_ciphertext[message_len] ^= 1;
            assert_eq!(
                unsafe {
                    c_decrypt(
                        c_plaintext.as_mut_ptr(),
                        &mut c_plaintext_len,
                        std::ptr::null_mut(),
                        c_ciphertext.as_ptr(),
                        c_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                },
                unsafe {
                    rust_decrypt(
                        rust_plaintext.as_mut_ptr(),
                        &mut rust_plaintext_len,
                        std::ptr::null_mut(),
                        rust_ciphertext.as_ptr(),
                        rust_len,
                        ad.as_ptr(),
                        ad_len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    )
                },
                "{name} tampered"
            );
        }
    }
}

#[test]
fn randomized_aead_outputs_and_tamper_rejections_match() {
    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        for (name, key_len, nonce_len, tag_len) in [
            ("crypto_aead_aegis128l", 16, 16, 32),
            ("crypto_aead_aegis256", 32, 32, 32),
            ("crypto_aead_chacha20poly1305", 32, 8, 16),
            ("crypto_aead_chacha20poly1305_ietf", 32, 12, 16),
            ("crypto_aead_xchacha20poly1305_ietf", 32, 24, 16),
        ] {
            compare_aead(&libraries, name, key_len, nonce_len, tag_len, &mut rng);
        }
    }
}

#[test]
fn deterministic_sign_box_scalarmult_and_kdf_paths_match() {
    type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    type Verify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
    type BoxEasy =
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
    type ScalarBase = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
    type Kdf = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let mut rng = Rng::new();
        let seed = rng.bytes(32);
        let (c_keypair, rust_keypair) = libraries.pair::<SeedKeypair>("crypto_sign_seed_keypair");
        let mut c_pk = vec![0u8; 32];
        let mut rust_pk = c_pk.clone();
        let mut c_sk = vec![0u8; 64];
        let mut rust_sk = c_sk.clone();
        assert_eq!(
            c_keypair(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr()),
            rust_keypair(rust_pk.as_mut_ptr(), rust_sk.as_mut_ptr(), seed.as_ptr())
        );
        assert_eq!(c_pk, rust_pk);
        assert_eq!(c_sk, rust_sk);

        let (c_sign, rust_sign) = libraries.pair::<Sign>("crypto_sign_detached");
        let (c_verify, rust_verify) = libraries.pair::<Verify>("crypto_sign_verify_detached");
        for len in lengths() {
            let message = rng.bytes(len);
            let mut c_sig = vec![0u8; 64];
            let mut rust_sig = c_sig.clone();
            let mut c_sig_len = 0;
            let mut rust_sig_len = 0;
            assert_eq!(
                c_sign(
                    c_sig.as_mut_ptr(),
                    &mut c_sig_len,
                    message.as_ptr(),
                    len as u64,
                    c_sk.as_ptr()
                ),
                rust_sign(
                    rust_sig.as_mut_ptr(),
                    &mut rust_sig_len,
                    message.as_ptr(),
                    len as u64,
                    rust_sk.as_ptr()
                )
            );
            assert_eq!(c_sig_len, rust_sig_len);
            assert_eq!(c_sig, rust_sig);
            assert_eq!(
                c_verify(c_sig.as_ptr(), message.as_ptr(), len as u64, c_pk.as_ptr()),
                rust_verify(
                    rust_sig.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    rust_pk.as_ptr()
                )
            );
        }

        let alice_seed = rng.bytes(32);
        let bob_seed = rng.bytes(32);
        let (c_box_keypair, rust_box_keypair) =
            libraries.pair::<SeedKeypair>("crypto_box_seed_keypair");
        let mut c_alice_pk = vec![0u8; 32];
        let mut c_alice_sk = vec![0u8; 32];
        let mut rust_alice_pk = c_alice_pk.clone();
        let mut rust_alice_sk = c_alice_sk.clone();
        let mut c_bob_pk = vec![0u8; 32];
        let mut c_bob_sk = vec![0u8; 32];
        let mut rust_bob_pk = c_bob_pk.clone();
        let mut rust_bob_sk = c_bob_sk.clone();
        assert_eq!(
            c_box_keypair(
                c_alice_pk.as_mut_ptr(),
                c_alice_sk.as_mut_ptr(),
                alice_seed.as_ptr()
            ),
            rust_box_keypair(
                rust_alice_pk.as_mut_ptr(),
                rust_alice_sk.as_mut_ptr(),
                alice_seed.as_ptr()
            )
        );
        assert_eq!(
            c_box_keypair(
                c_bob_pk.as_mut_ptr(),
                c_bob_sk.as_mut_ptr(),
                bob_seed.as_ptr()
            ),
            rust_box_keypair(
                rust_bob_pk.as_mut_ptr(),
                rust_bob_sk.as_mut_ptr(),
                bob_seed.as_ptr()
            )
        );
        assert_eq!(c_alice_pk, rust_alice_pk);
        assert_eq!(c_bob_pk, rust_bob_pk);
        let nonce = rng.bytes(24);
        let message = rng.bytes(97);
        let (c_box, rust_box) = libraries.pair::<BoxEasy>("crypto_box_easy");
        let (c_open, rust_open) = libraries.pair::<BoxEasy>("crypto_box_open_easy");
        let mut c_ciphertext = vec![0u8; message.len() + 16];
        let mut rust_ciphertext = c_ciphertext.clone();
        assert_eq!(
            c_box(
                c_ciphertext.as_mut_ptr(),
                message.as_ptr(),
                message.len() as u64,
                nonce.as_ptr(),
                c_bob_pk.as_ptr(),
                c_alice_sk.as_ptr()
            ),
            rust_box(
                rust_ciphertext.as_mut_ptr(),
                message.as_ptr(),
                message.len() as u64,
                nonce.as_ptr(),
                rust_bob_pk.as_ptr(),
                rust_alice_sk.as_ptr()
            )
        );
        assert_eq!(c_ciphertext, rust_ciphertext);
        let mut c_plaintext = vec![0u8; message.len()];
        let mut rust_plaintext = c_plaintext.clone();
        assert_eq!(
            c_open(
                c_plaintext.as_mut_ptr(),
                c_ciphertext.as_ptr(),
                c_ciphertext.len() as u64,
                nonce.as_ptr(),
                c_alice_pk.as_ptr(),
                c_bob_sk.as_ptr()
            ),
            rust_open(
                rust_plaintext.as_mut_ptr(),
                rust_ciphertext.as_ptr(),
                rust_ciphertext.len() as u64,
                nonce.as_ptr(),
                rust_alice_pk.as_ptr(),
                rust_bob_sk.as_ptr()
            )
        );
        assert_eq!(c_plaintext, rust_plaintext);
        assert_eq!(c_plaintext, message);

        let scalar = rng.bytes(32);
        let (c_base, rust_base) = libraries.pair::<ScalarBase>("crypto_scalarmult_curve25519_base");
        let mut c_point = vec![0u8; 32];
        let mut rust_point = c_point.clone();
        assert_eq!(
            c_base(c_point.as_mut_ptr(), scalar.as_ptr()),
            rust_base(rust_point.as_mut_ptr(), scalar.as_ptr())
        );
        assert_eq!(c_point, rust_point);

        let (c_kdf, rust_kdf) = libraries.pair::<Kdf>("crypto_kdf_blake2b_derive_from_key");
        let key = rng.bytes(32);
        let context = *b"DiffTest";
        for output_len in [16, 17, 32, 63, 64] {
            for id in [0, 1, u32::MAX as u64, u64::MAX] {
                let mut c_out = vec![0u8; output_len];
                let mut rust_out = c_out.clone();
                assert_eq!(
                    c_kdf(
                        c_out.as_mut_ptr(),
                        output_len,
                        id,
                        context.as_ptr().cast(),
                        key.as_ptr()
                    ),
                    rust_kdf(
                        rust_out.as_mut_ptr(),
                        output_len,
                        id,
                        context.as_ptr().cast(),
                        key.as_ptr()
                    )
                );
                assert_eq!(c_out, rust_out);
            }
        }
    }
}

#[test]
fn exact_error_codes_match_for_boundaries_and_invalid_values() {
    type GenericHash =
        unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
    type Kdf = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> i32;
    type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
    type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;
    type Pwhash =
        unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, i32) -> i32;
    type AegisDecrypt = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        u64,
        *const u8,
        *const u8,
        u64,
        *const u8,
        *const u8,
    ) -> i32;

    unsafe {
        let libraries = Libraries::open();
        initialize(&libraries);
        let input = [0u8; 64];
        let key = [0u8; 64];
        let mut c_out = [0xa5u8; 80];
        let mut rust_out = c_out;

        let (c_hash, rust_hash) = libraries.pair::<GenericHash>("crypto_generichash");
        for (output_len, key_len, expected) in [
            (0, 0, -1),
            (15, 0, 0),
            (65, 0, -1),
            (32, 1, 0),
            (32, 65, -1),
        ] {
            let c_result = c_hash(
                c_out.as_mut_ptr(),
                output_len,
                input.as_ptr(),
                1,
                key.as_ptr(),
                key_len,
            );
            let rust_result = rust_hash(
                rust_out.as_mut_ptr(),
                output_len,
                input.as_ptr(),
                1,
                key.as_ptr(),
                key_len,
            );
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, expected);
        }

        let (c_kdf, rust_kdf) = libraries.pair::<Kdf>("crypto_kdf_blake2b_derive_from_key");
        let context = *b"DiffTest";
        for output_len in [0, 15, 65, usize::MAX] {
            let c_result = c_kdf(
                c_out.as_mut_ptr(),
                output_len,
                0,
                context.as_ptr().cast(),
                key.as_ptr(),
            );
            let rust_result = rust_kdf(
                rust_out.as_mut_ptr(),
                output_len,
                0,
                context.as_ptr().cast(),
                key.as_ptr(),
            );
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, -1);
        }

        let (c_pad, rust_pad) = libraries.pair::<Pad>("sodium_pad");
        let (c_unpad, rust_unpad) = libraries.pair::<Unpad>("sodium_unpad");
        let mut c_len = usize::MAX;
        let mut rust_len = usize::MAX;
        for block in [0, usize::MAX] {
            assert_eq!(
                c_pad(&mut c_len, c_out.as_mut_ptr(), 1, block, c_out.len()),
                rust_pad(
                    &mut rust_len,
                    rust_out.as_mut_ptr(),
                    1,
                    block,
                    rust_out.len()
                )
            );
            assert_eq!(
                c_unpad(&mut c_len, c_out.as_ptr(), 0, block),
                rust_unpad(&mut rust_len, rust_out.as_ptr(), 0, block)
            );
        }
        assert_eq!(
            c_pad(&mut c_len, c_out.as_mut_ptr(), 8, 16, 8),
            rust_pad(&mut rust_len, rust_out.as_mut_ptr(), 8, 16, 8)
        );

        let (c_pwhash, rust_pwhash) = libraries.pair::<Pwhash>("crypto_pwhash");
        let salt = [0u8; 16];
        let c_result = c_pwhash(
            c_out.as_mut_ptr(),
            32,
            b"password".as_ptr().cast(),
            8,
            salt.as_ptr(),
            1,
            8192,
            i32::MAX,
        );
        let rust_result = rust_pwhash(
            rust_out.as_mut_ptr(),
            32,
            b"password".as_ptr().cast(),
            8,
            salt.as_ptr(),
            1,
            8192,
            i32::MAX,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, -1);

        for (name, nonce_len) in [
            ("crypto_aead_aegis128l_decrypt_detached", 16),
            ("crypto_aead_aegis256_decrypt_detached", 32),
        ] {
            let (c, rust) = libraries.pair::<AegisDecrypt>(name);
            let nonce = vec![0u8; nonce_len];
            let c_result = c(
                c_out.as_mut_ptr(),
                std::ptr::null_mut(),
                input.as_ptr(),
                1u64 << 61,
                input.as_ptr(),
                input.as_ptr(),
                0,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            let rust_result = rust(
                rust_out.as_mut_ptr(),
                std::ptr::null_mut(),
                input.as_ptr(),
                1u64 << 61,
                input.as_ptr(),
                input.as_ptr(),
                0,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            assert_eq!(c_result, rust_result, "{name}");
            assert_eq!(c_result, -1, "{name}");
        }
    }
}
