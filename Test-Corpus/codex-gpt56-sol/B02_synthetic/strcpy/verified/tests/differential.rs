use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;

use libloading::Library;

type ProcessStrings =
    unsafe extern "C" fn(*mut c_char, usize, *const c_char, usize, c_int, u32) -> c_int;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c: ProcessStrings,
    rust: ProcessStrings,
}

impl Apis {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver_c.so");
        let test_exe = std::env::current_exe().expect("current test executable");
        let profile_dir = test_exe
            .parent()
            .and_then(|deps| deps.parent())
            .expect("Cargo profile directory");
        let rust_path = profile_dir.join("libdriver.so");

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c = *c_library
                .get::<ProcessStrings>(b"process_strings\0")
                .expect("load C process_strings");
            let rust = *rust_library
                .get::<ProcessStrings>(b"process_strings\0")
                .expect("load Rust process_strings");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn compare(
        &self,
        label: &str,
        input: Option<&mut [u8]>,
        input_len: usize,
        reference: Option<&[u8]>,
        ref_len: usize,
        operation: c_int,
        flags: u32,
        expected: c_int,
    ) {
        let input_ptr = match input {
            Some(bytes) => bytes.as_mut_ptr().cast(),
            None => ptr::null_mut(),
        };
        let reference_ptr = match reference {
            Some(bytes) => bytes.as_ptr().cast(),
            None => ptr::null(),
        };

        let c_result = unsafe {
            (self.c)(
                input_ptr,
                input_len,
                reference_ptr,
                ref_len,
                operation,
                flags,
            )
        };
        let rust_result = unsafe {
            (self.rust)(
                input_ptr,
                input_len,
                reference_ptr,
                ref_len,
                operation,
                flags,
            )
        };

        assert_eq!(c_result, expected, "{label}: unexpected C result");
        assert_eq!(rust_result, c_result, "{label}: Rust differs from C");
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u32() as usize) % upper_exclusive
    }

    fn ascii(&mut self, max_len: usize) -> Vec<u8> {
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let len = self.usize(max_len + 1);
        (0..len)
            .map(|_| ALPHABET[self.usize(ALPHABET.len())])
            .collect()
    }

    fn nonempty_ascii(&mut self, max_len: usize) -> Vec<u8> {
        let mut value = self.ascii(max_len.saturating_sub(1));
        value.push(b'a' + self.usize(26) as u8);
        value
    }
}

fn nul_terminated(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

fn wildcard(prefix: &[u8], pattern: &[u8], suffix: &[u8]) -> Vec<u8> {
    prefix
        .iter()
        .chain(pattern)
        .chain(suffix)
        .take(63)
        .copied()
        .collect()
}

#[test]
fn v01_validate_exact_randomized() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0101_5eed);
    for iteration in 0..128 {
        let value = rng.ascii(96);
        let mut input = nul_terminated(value.clone());
        let reference = nul_terminated(value);
        api.compare(
            &format!("V01 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            0,
            rng.next_u32(),
            1,
        );
    }
}

#[test]
fn v02_validate_fallback_tokens_randomized() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0202_5eed);
    for iteration in 0..128 {
        let token: &[u8] = if iteration % 2 == 0 { b"VALID" } else { b"OK" };
        let mut input = nul_terminated(token.to_vec());
        let reference = nul_terminated(rng.nonempty_ascii(80));
        api.compare(
            &format!("V02 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            0,
            rng.next_u32(),
            1,
        );
    }
}

#[test]
fn v03_standard_commands_guarded_path() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0303_5eed);
    let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
    for iteration in 0..128 {
        let index = rng.usize(commands.len());
        let mut input = nul_terminated(commands[index].to_vec());
        api.compare(
            &format!("V03 iteration {iteration}"),
            Some(&mut input),
            commands[index].len() + rng.usize(64),
            None,
            rng.usize(64),
            1,
            rng.next_u32(),
            index as c_int,
        );
    }
}

#[test]
fn v04_standard_commands_with_space() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0404_5eed);
    let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
    for iteration in 0..128 {
        let index = rng.usize(commands.len());
        let mut bytes = commands[index].to_vec();
        bytes.push(b' ');
        bytes.extend(rng.ascii(40));
        let mut input = nul_terminated(bytes);
        api.compare(
            &format!("V04 iteration {iteration}"),
            Some(&mut input),
            commands[index].len() + rng.usize(64),
            None,
            0,
            1,
            rng.next_u32(),
            index as c_int,
        );
    }
}

#[test]
fn v05_standard_commands_fallback_strcmp() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0505_5eed);
    let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
    for iteration in 0..128 {
        let index = rng.usize(commands.len());
        let mut input = nul_terminated(commands[index].to_vec());
        api.compare(
            &format!("V05 iteration {iteration}"),
            Some(&mut input),
            rng.usize(commands[index].len()),
            None,
            0,
            1,
            rng.next_u32(),
            index as c_int,
        );
    }
}

#[test]
fn v06_admin_command_randomized_lengths() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0606_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(b"ADMIN".to_vec());
        api.compare(
            &format!("V06 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            None,
            rng.usize(128),
            1,
            rng.next_u32(),
            99,
        );
    }
}

#[test]
fn v07_prefix_mode_matches_equal_and_longer_inputs() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0707_5eed);
    for iteration in 0..128 {
        let prefix = rng.ascii(64);
        let mut value = prefix.clone();
        value.extend(rng.ascii(48));
        let mut input = nul_terminated(value);
        let reference = nul_terminated(prefix);
        api.compare(
            &format!("V07 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            2,
            rng.next_u32() & !1,
            1,
        );
    }
}

#[test]
fn v08_exact_prefix_mode_matches_reference() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0808_5eed);
    for iteration in 0..128 {
        let value = rng.ascii(96);
        let mut input = nul_terminated(value.clone());
        let reference = nul_terminated(value);
        api.compare(
            &format!("V08 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            2,
            rng.next_u32() | 1,
            1,
        );
    }
}

#[test]
fn v09_exact_prefix_suffix_variations() {
    let api = Apis::load();
    let mut rng = Rng::new(0x0909_5eed);
    let suffixes: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
    for iteration in 0..160 {
        let suffix_index = rng.usize(suffixes.len());
        let prefix = rng.ascii(48);
        let mut value = prefix.clone();
        value.extend_from_slice(suffixes[suffix_index]);
        let mut input = nul_terminated(value);
        let reference = nul_terminated(prefix);
        api.compare(
            &format!("V09 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            2,
            rng.next_u32() | 1,
            2 + suffix_index as c_int,
        );
    }
}

#[test]
fn v10_exact_prefix_truncates_at_63_bytes() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1010_5eed);
    for iteration in 0..128 {
        let len = 64 + rng.usize(96);
        let reference_bytes: Vec<u8> = (0..len)
            .map(|index| b'a' + ((index + rng.usize(26)) % 26) as u8)
            .collect();
        let mut input = nul_terminated(reference_bytes[..63].to_vec());
        let reference = nul_terminated(reference_bytes);
        api.compare(
            &format!("V10 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            2,
            rng.next_u32() | 1,
            2,
        );
    }
}

#[test]
fn v11_explicit_delimiter_positions() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1111_5eed);
    for iteration in 0..128 {
        let len = 1 + rng.usize(128);
        let position = match iteration % 3 {
            0 => 0,
            1 => len / 2,
            _ => len - 1,
        };
        let delimiter = b'|';
        let mut bytes = vec![b'a'; len];
        bytes[position] = delimiter;
        let mut input = nul_terminated(bytes);
        let reference = [delimiter, 0];
        api.compare(
            &format!("V11 iteration {iteration}"),
            Some(&mut input),
            len,
            Some(&reference),
            1 + rng.usize(64),
            3,
            rng.next_u32(),
            position as c_int,
        );
    }
}

#[test]
fn v12_null_reference_selects_default_delimiter() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1212_5eed);
    for iteration in 0..128 {
        let position = rng.usize(96);
        let mut bytes = vec![b'a'; position + 1 + rng.usize(24)];
        bytes[position] = b':';
        let len = bytes.len();
        let mut input = nul_terminated(bytes);
        api.compare(
            &format!("V12 iteration {iteration}"),
            Some(&mut input),
            len,
            None,
            rng.usize(128),
            3,
            rng.next_u32(),
            position as c_int,
        );
    }
}

#[test]
fn v13_zero_reference_length_selects_default_delimiter() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1313_5eed);
    for iteration in 0..128 {
        let position = rng.usize(96);
        let mut bytes = vec![b'a'; position + 1 + rng.usize(24)];
        bytes[position] = b':';
        let len = bytes.len();
        let mut input = nul_terminated(bytes);
        let ignored_reference = [b'|'];
        api.compare(
            &format!("V13 iteration {iteration}"),
            Some(&mut input),
            len,
            Some(&ignored_reference),
            0,
            3,
            rng.next_u32(),
            position as c_int,
        );
    }
}

#[test]
fn v14_embedded_nul_stops_delimiter_search() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1414_5eed);
    for iteration in 0..128 {
        let nul_position = rng.usize(64);
        let mut bytes = vec![b'a'; nul_position];
        bytes.push(0);
        bytes.extend(rng.ascii(32));
        bytes.push(b'|');
        bytes.push(0);
        let len = bytes.len();
        let reference = [b'|', 0];
        api.compare(
            &format!("V14 iteration {iteration}"),
            Some(&mut bytes),
            len,
            Some(&reference),
            1,
            3,
            rng.next_u32(),
            -1,
        );
    }
}

#[test]
fn v15_binary_and_oversized_allocated_delimiter_lengths() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1515_5eed);
    for iteration in 0..64 {
        let len = if iteration == 0 {
            2048
        } else {
            1 + rng.usize(512)
        };
        let mut bytes: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u32() % 126) as u8).collect();
        bytes.push(0);
        let reference = [0xff, 0];
        api.compare(
            &format!("V15 iteration {iteration}"),
            Some(&mut bytes),
            len,
            Some(&reference),
            1,
            3,
            rng.next_u32(),
            -1,
        );
    }
}

#[test]
fn v16_case_insensitive_mode_exact_matches() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1616_5eed);
    for iteration in 0..128 {
        let value = rng.ascii(96);
        let mut input = nul_terminated(value.clone());
        let reference = nul_terminated(value);
        api.compare(
            &format!("V16 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            4,
            rng.next_u32() & !2,
            1,
        );
    }
}

#[test]
fn v17_case_insensitive_mode_prefix_result() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1717_5eed);
    for iteration in 0..128 {
        let pattern = rng.nonempty_ascii(48);
        let mut text = pattern.clone();
        text.extend(rng.nonempty_ascii(32));
        let mut input = nul_terminated(text);
        let reference = nul_terminated(pattern);
        api.compare(
            &format!("V17 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            4,
            rng.next_u32() & !2,
            5,
        );
    }
}

#[test]
fn v18_ascii_case_folded_equal_length_matches() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1818_5eed);
    for iteration in 0..128 {
        let len = 1 + rng.usize(80);
        let lower: Vec<u8> = (0..len).map(|_| b'a' + rng.usize(26) as u8).collect();
        let upper: Vec<u8> = lower.iter().map(u8::to_ascii_uppercase).collect();
        let mut input = nul_terminated(lower);
        let reference = nul_terminated(upper);
        api.compare(
            &format!("V18 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            4,
            rng.next_u32() & !2,
            6,
        );
    }
}

#[test]
fn v19_case_sensitive_mode_exact_matches() {
    let api = Apis::load();
    let mut rng = Rng::new(0x1919_5eed);
    for iteration in 0..128 {
        let value = rng.ascii(96);
        let mut input = nul_terminated(value.clone());
        let reference = nul_terminated(value);
        api.compare(
            &format!("V19 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            4,
            rng.next_u32() | 2,
            1,
        );
    }
}

#[test]
fn v20_case_sensitive_wildcard_forms() {
    let api = Apis::load();
    let mut rng = Rng::new(0x2020_5eed);
    for iteration in 0..128 {
        let pattern = rng.nonempty_ascii(48);
        for (form, (prefix, suffix)) in [
            (b"*".as_slice(), b"*".as_slice()),
            (b"".as_slice(), b"*".as_slice()),
            (b"*".as_slice(), b"".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            let mut text = prefix.to_vec();
            text.extend_from_slice(&pattern);
            text.extend_from_slice(suffix);
            let mut input = nul_terminated(text);
            let reference = nul_terminated(pattern.clone());
            api.compare(
                &format!("V20 iteration {iteration} form {form}"),
                Some(&mut input),
                rng.usize(256),
                Some(&reference),
                rng.usize(256),
                4,
                rng.next_u32() | 2,
                2 + form as c_int,
            );
        }
    }
}

#[test]
fn v21_case_sensitive_substring_positions() {
    let api = Apis::load();
    let mut rng = Rng::new(0x2121_5eed);
    for iteration in 0..128 {
        let pattern = rng.nonempty_ascii(24);
        let prefix_len = match iteration % 3 {
            0 => 0,
            1 => 1 + rng.usize(32),
            _ => 1 + rng.usize(32),
        };
        let suffix_len = if iteration % 3 == 2 {
            0
        } else {
            1 + rng.usize(32)
        };
        let mut text = vec![b'!'; prefix_len];
        text.extend_from_slice(&pattern);
        text.extend(vec![b'?'; suffix_len]);
        let mut input = nul_terminated(text);
        let reference = nul_terminated(pattern);
        api.compare(
            &format!("V21 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            Some(&reference),
            rng.usize(256),
            4,
            rng.next_u32() | 2,
            10 + prefix_len as c_int,
        );
    }
}

#[test]
fn v22_wildcards_at_snprintf_boundary() {
    let api = Apis::load();
    let mut rng = Rng::new(0x2222_5eed);
    for pattern_len in 60..=72 {
        for (form, (prefix, suffix)) in [
            (b"*".as_slice(), b"*".as_slice()),
            (b"".as_slice(), b"*".as_slice()),
            (b"*".as_slice(), b"".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            for iteration in 0..16 {
                let pattern: Vec<u8> = (0..pattern_len)
                    .map(|index| b'a' + ((index + iteration) % 26) as u8)
                    .collect();
                let text = wildcard(prefix, &pattern, suffix);
                let generated = [
                    wildcard(b"*", &pattern, b"*"),
                    wildcard(b"", &pattern, b"*"),
                    wildcard(b"*", &pattern, b""),
                ];
                let expected = if text == pattern {
                    1
                } else {
                    2 + generated
                        .iter()
                        .position(|candidate| candidate == &text)
                        .expect("generated wildcard form") as c_int
                };
                let mut input = nul_terminated(text);
                let reference = nul_terminated(pattern);
                api.compare(
                    &format!("V22 len {pattern_len} form {form} iteration {iteration}"),
                    Some(&mut input),
                    rng.usize(256),
                    Some(&reference),
                    rng.usize(256),
                    4,
                    rng.next_u32() | 2,
                    expected,
                );
            }
        }
    }
}

#[test]
fn v23_embedded_nul_terminates_all_string_operations() {
    let api = Apis::load();
    let mut rng = Rng::new(0x2323_5eed);
    for iteration in 0..128 {
        let tail_a = rng.nonempty_ascii(32);
        let tail_b = rng.nonempty_ascii(32);

        let mut op0_input = b"same\0".to_vec();
        op0_input.extend_from_slice(&tail_a);
        let mut op0_ref = b"same\0".to_vec();
        op0_ref.extend_from_slice(&tail_b);
        api.compare(
            &format!("V23 op0 iteration {iteration}"),
            Some(&mut op0_input),
            rng.usize(128),
            Some(&op0_ref),
            rng.usize(128),
            0,
            rng.next_u32(),
            1,
        );

        let mut op1_input = b"START\0".to_vec();
        op1_input.extend_from_slice(&tail_a);
        api.compare(
            &format!("V23 op1 iteration {iteration}"),
            Some(&mut op1_input),
            64,
            None,
            0,
            1,
            rng.next_u32(),
            0,
        );

        let mut op2_input = b"pre\0".to_vec();
        op2_input.extend_from_slice(&tail_a);
        let mut op2_ref = b"pre\0".to_vec();
        op2_ref.extend_from_slice(&tail_b);
        api.compare(
            &format!("V23 op2 iteration {iteration}"),
            Some(&mut op2_input),
            rng.usize(128),
            Some(&op2_ref),
            rng.usize(128),
            2,
            rng.next_u32(),
            1,
        );

        let mut op3_input = b"a\0:\0".to_vec();
        api.compare(
            &format!("V23 op3 iteration {iteration}"),
            Some(&mut op3_input),
            4,
            None,
            0,
            3,
            rng.next_u32(),
            -1,
        );

        let mut op4_input = b"match\0".to_vec();
        op4_input.extend_from_slice(&tail_a);
        let mut op4_ref = b"match\0".to_vec();
        op4_ref.extend_from_slice(&tail_b);
        api.compare(
            &format!("V23 op4 iteration {iteration}"),
            Some(&mut op4_input),
            rng.usize(128),
            Some(&op4_ref),
            rng.usize(128),
            4,
            rng.next_u32(),
            1,
        );
    }
}

#[test]
fn v24_only_documented_flag_bits_change_behavior() {
    let api = Apis::load();
    let flag_values = [0, 1, 2, 4, 8, 0x8000_0000, u32::MAX];
    for flags in flag_values {
        let mut op0 = nul_terminated(b"same".to_vec());
        let op0_ref = nul_terminated(b"same".to_vec());
        api.compare("V24 op0", Some(&mut op0), 4, Some(&op0_ref), 4, 0, flags, 1);

        let mut op1 = nul_terminated(b"STOP".to_vec());
        api.compare("V24 op1", Some(&mut op1), 4, None, 0, 1, flags, 1);

        let mut op3 = nul_terminated(b"a:b".to_vec());
        api.compare("V24 op3", Some(&mut op3), 3, None, 0, 3, flags, 1);
    }

    for flags in [0, 2, 4, 0x8000_0000, u32::MAX & !1] {
        let mut input = nul_terminated(b"prefix-tail".to_vec());
        let reference = nul_terminated(b"prefix".to_vec());
        api.compare(
            "V24 op2 bit clear",
            Some(&mut input),
            0,
            Some(&reference),
            0,
            2,
            flags,
            1,
        );
    }
    for flags in [1, 3, 5, u32::MAX] {
        let mut input = nul_terminated(b"prefix-tail".to_vec());
        let reference = nul_terminated(b"prefix".to_vec());
        api.compare(
            "V24 op2 bit set",
            Some(&mut input),
            0,
            Some(&reference),
            0,
            2,
            flags,
            0,
        );
    }

    for flags in [0, 1, 4, 0x8000_0000, u32::MAX & !2] {
        let mut input = nul_terminated(b"abc".to_vec());
        let reference = nul_terminated(b"ABC".to_vec());
        api.compare(
            "V24 op4 bit clear",
            Some(&mut input),
            0,
            Some(&reference),
            0,
            4,
            flags,
            6,
        );
    }
    for flags in [2, 3, 6, u32::MAX] {
        let mut input = nul_terminated(b"abc".to_vec());
        let reference = nul_terminated(b"ABC".to_vec());
        api.compare(
            "V24 op4 bit set",
            Some(&mut input),
            0,
            Some(&reference),
            0,
            4,
            flags,
            0,
        );
    }
}

#[test]
fn e01_null_input_precedes_dispatch() {
    let api = Apis::load();
    let reference = nul_terminated(b"reference".to_vec());
    for operation in [c_int::MIN, -1, 0, 1, 2, 3, 4, 5, c_int::MAX] {
        api.compare(
            "E01",
            None,
            usize::MAX,
            Some(&reference),
            usize::MAX,
            operation,
            u32::MAX,
            -1,
        );
    }
}

#[test]
fn e02_validate_rejects_null_reference() {
    let api = Apis::load();
    let mut input = nul_terminated(b"input".to_vec());
    api.compare("E02", Some(&mut input), 5, None, 0, 0, 0, -2);
}

#[test]
fn e03_prefix_rejects_null_reference() {
    let api = Apis::load();
    let mut input = nul_terminated(b"input".to_vec());
    api.compare("E03", Some(&mut input), 5, None, 0, 2, u32::MAX, -2);
}

#[test]
fn e04_pattern_rejects_null_reference() {
    let api = Apis::load();
    let mut input = nul_terminated(b"input".to_vec());
    api.compare("E04", Some(&mut input), 5, None, 0, 4, u32::MAX, -2);
}

#[test]
fn e05_unknown_operations_return_minus_three() {
    let api = Apis::load();
    let reference = nul_terminated(b"reference".to_vec());
    for operation in [c_int::MIN, -100, -1, 5, 6, 100, c_int::MAX] {
        let mut input = nul_terminated(b"input".to_vec());
        api.compare(
            &format!("E05 operation {operation}"),
            Some(&mut input),
            usize::MAX,
            Some(&reference),
            usize::MAX,
            operation,
            u32::MAX,
            -3,
        );
    }
}

#[test]
fn e06_invalid_tokens_return_zero() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe606_5eed);
    for iteration in 0..128 {
        let mut value = rng.nonempty_ascii(64);
        if value == b"VALID" || value == b"OK" {
            value.push(b'x');
        }
        let mut input = nul_terminated(value);
        let reference = nul_terminated(b"different-reference".to_vec());
        api.compare(
            &format!("E06 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            Some(&reference),
            rng.usize(128),
            0,
            rng.next_u32(),
            0,
        );
    }
}

#[test]
fn e07_unknown_commands_return_minus_one() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe707_5eed);
    for iteration in 0..128 {
        let value = format!("UNKNOWN{iteration}").into_bytes();
        let mut input = nul_terminated(value);
        api.compare(
            &format!("E07 iteration {iteration}"),
            Some(&mut input),
            rng.usize(256),
            None,
            0,
            1,
            rng.next_u32(),
            -1,
        );
    }
}

#[test]
fn e08_prefix_mode_nonmatches_return_zero() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe808_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(format!("left{iteration}").into_bytes());
        let reference = nul_terminated(format!("right{iteration}").into_bytes());
        api.compare(
            &format!("E08 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            Some(&reference),
            rng.usize(128),
            2,
            rng.next_u32() & !1,
            0,
        );
    }
}

#[test]
fn e09_exact_prefix_mode_nonmatches_return_zero() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe909_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(format!("left{iteration}").into_bytes());
        let reference = nul_terminated(format!("right{iteration}").into_bytes());
        api.compare(
            &format!("E09 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            Some(&reference),
            rng.usize(128),
            2,
            rng.next_u32() | 1,
            0,
        );
    }
}

#[test]
fn e10_zero_delimiter_length_returns_minus_one() {
    let api = Apis::load();
    let mut input = nul_terminated(b":present".to_vec());
    let reference = [b':', 0];
    api.compare("E10", Some(&mut input), 0, Some(&reference), 1, 3, 0, -1);
}

#[test]
fn e11_none_pipe_sentinel_returns_minus_two() {
    let api = Apis::load();
    let mut input = nul_terminated(b"NONE".to_vec());
    let reference = [b'|', 0];
    api.compare("E11", Some(&mut input), 4, Some(&reference), 1, 3, 0, -2);
}

#[test]
fn e12_empty_colon_sentinel_returns_minus_three() {
    let api = Apis::load();
    let mut input = nul_terminated(b"EMPTY".to_vec());
    api.compare("E12", Some(&mut input), 5, None, 0, 3, 0, -3);
}

#[test]
fn e13_missing_delimiters_return_minus_one() {
    let api = Apis::load();
    let mut rng = Rng::new(0xed13_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(format!("value{iteration}").into_bytes());
        let reference = [b'|', 0];
        let input_len = input.len() - 1;
        api.compare(
            &format!("E13 iteration {iteration}"),
            Some(&mut input),
            input_len,
            Some(&reference),
            1,
            3,
            rng.next_u32(),
            -1,
        );
    }
}

#[test]
fn e14_case_sensitive_pattern_nonmatches_return_zero() {
    let api = Apis::load();
    let mut rng = Rng::new(0xee14_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(format!("abcdefgh{iteration}").into_bytes());
        let reference = nul_terminated(b"XYZ".to_vec());
        api.compare(
            &format!("E14 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            Some(&reference),
            rng.usize(128),
            4,
            rng.next_u32() | 2,
            0,
        );
    }
}

#[test]
fn e15_case_insensitive_pattern_nonmatches_return_zero() {
    let api = Apis::load();
    let mut rng = Rng::new(0xef15_5eed);
    for iteration in 0..128 {
        let mut input = nul_terminated(b"abc".to_vec());
        let reference = nul_terminated(b"XYZ".to_vec());
        api.compare(
            &format!("E15 iteration {iteration}"),
            Some(&mut input),
            rng.usize(128),
            Some(&reference),
            rng.usize(128),
            4,
            rng.next_u32() & !2,
            0,
        );
    }
}

#[test]
fn generic_length_and_ffi_boundaries() {
    let api = Apis::load();

    let mut delimiter_first = nul_terminated(b":rest".to_vec());
    let reference = [b':', 0];
    api.compare(
        "oversized input_len with immediate delimiter",
        Some(&mut delimiter_first),
        usize::MAX,
        Some(&reference),
        usize::MAX,
        3,
        u32::MAX,
        0,
    );

    let mut default_delimiter = nul_terminated(b":rest".to_vec());
    api.compare(
        "oversized ref_len with null reference",
        Some(&mut default_delimiter),
        1,
        None,
        usize::MAX,
        3,
        u32::MAX,
        0,
    );

    for length in [0, usize::MAX] {
        for operation in [0, 1, 2, 4] {
            let mut input = nul_terminated(b"same".to_vec());
            let reference = nul_terminated(b"same".to_vec());
            let expected = if operation == 1 { -1 } else { 1 };
            api.compare(
                &format!("boundary length {length} operation {operation}"),
                Some(&mut input),
                length,
                Some(&reference),
                length,
                operation,
                0,
                expected,
            );
        }
    }

    let mut one_byte = vec![b':', 0];
    api.compare(
        "one-byte searched input",
        Some(&mut one_byte),
        1,
        None,
        0,
        3,
        0,
        0,
    );
}
