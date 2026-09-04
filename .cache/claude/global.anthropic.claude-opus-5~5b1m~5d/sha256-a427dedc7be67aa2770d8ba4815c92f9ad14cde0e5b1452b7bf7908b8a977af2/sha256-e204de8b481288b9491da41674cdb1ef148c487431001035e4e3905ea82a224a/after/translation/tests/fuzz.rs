//! Deterministic randomised differential testing.
//!
//! The hand-written cases in the other test files cover the branches that were
//! enumerated from the C source; this file sweeps the space between them.  The
//! generator is seeded with fixed values so a failure is always reproducible.

mod common;

use common::{assert_same, data_dir, data_path};

/// xorshift64*, so the suite needs no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const TOKENS: &[&str] = &[
    "int", "float", "if", "else", "while", "return", "sizeof", "typedef", "x",
    "y", "foo", "bar_1", "_ident", "0", "42", "3.14", "1.2.3", ".5", "0.",
    "9999999999999999999999", "+", "-", "*", "/", "%", "==", "!=", "<=", ">=",
    "&&", "||", "++", "--", "->", "<<", ">>", "=", "<", ">", "!", "&", "|", "^",
    "~", "?", ":", "(", ")", "{", "}", "[", "]", ";", ",", ".", "@", "#", "$",
    "`", "\\", "\"str\"", "'c'", "\"esc\\\"q\"", "'", "\"", "//comment",
    "/*ml*/", "/*open", "*/", "\t", "\u{b}", "\u{c}", "\r",
];

const ODD_CHOICES: &[&str] = &[
    "abc\n", "\n", "  \n", "1.7\n", "3q\n", "+2\n", "-0\n", "0x2\n",
    "99999999999999999999\n", "-99999999999999999999\n", "\t5\n", "\r\n",
    "2147483648\n", "4294967297\n",
];

const PATTERNS: &[&str] = &[
    "", "x", "int", "\"", "/", "  ", "=", "==", ";", "foo", "\u{ff}", "1",
];

fn random_text(rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::new();
    let lines = rng.below(7);
    for _ in 0..lines {
        let n = rng.below(13);
        let mut line = String::new();
        for i in 0..n {
            if i > 0 {
                line.push(' ');
            }
            line.push_str(rng.pick(TOKENS));
        }
        if rng.below(10) == 0 {
            let repeat = 1 + rng.below(40);
            line = line.repeat(repeat);
        }
        if rng.below(20) == 0 {
            line.push_str(&"a".repeat(300));
        }
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    // The empty line that terminates the text entry loop.
    out.push(b'\n');
    out
}

fn random_session(rng: &mut Rng, files: &[String]) -> Vec<u8> {
    let mut input = Vec::new();
    let rounds = 1 + rng.below(8);
    for _ in 0..rounds {
        if rng.below(12) == 0 {
            input.extend_from_slice(rng.pick(ODD_CHOICES).as_bytes());
            continue;
        }
        let choice = *rng.pick(&[1i32, 1, 2, 3, 4, 5, 6, 6, 0, 8, 99, -3]);
        input.extend_from_slice(format!("{choice}\n").as_bytes());
        match choice {
            1 | 6 => input.extend_from_slice(&random_text(rng)),
            2 => {
                input.extend_from_slice(rng.pick(files).as_bytes());
                input.push(b'\n');
            }
            5 => {
                input.extend_from_slice(rng.pick(PATTERNS).as_bytes());
                input.push(b'\n');
            }
            _ => {}
        }
    }
    if rng.below(10) < 7 {
        input.extend_from_slice(b"7\n");
    }
    input
}

fn fixture_paths() -> Vec<String> {
    let mut files: Vec<String> = [
        "small.c",
        "empty.txt",
        "one_word.txt",
        "newlines.txt",
        "size8191.txt",
        "size8192.txt",
        "size8193.txt",
        "size4096.txt",
        "nul_middle.bin",
        "nul_first.bin",
        "high_bytes.bin",
        "code.c",
        "a_directory",
        "no_permission.txt",
    ]
    .iter()
    .map(|name| data_path(name))
    .collect();
    files.push(data_dir().to_string_lossy().into_owned());
    files.push("/nonexistent".to_string());
    files.push(String::new());
    files.push("   ".to_string());
    files.push("/proc/self/status".to_string());
    files.push("/dev/null".to_string());
    files
}

#[test]
fn random_sessions() {
    let files = fixture_paths();
    for seed in [1u64, 7, 31, 512, 4242] {
        let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for iteration in 0..60 {
            let input = random_session(&mut rng, &files);
            assert_same(&format!("session/seed{seed}/{iteration}"), &input);
        }
    }
}

#[test]
fn random_bytes() {
    // Arbitrary binary input: whatever the choice parser and the tokenizer make
    // of it, both programs must agree.
    for seed in [11u64, 23, 97] {
        let mut rng = Rng::new(seed.wrapping_mul(0xD1B5_4A32_D192_ED03));
        for iteration in 0..60 {
            let n = rng.below(400);
            let mut input = Vec::with_capacity(n);
            for _ in 0..n {
                input.push((rng.next_u64() & 0xff) as u8);
            }
            assert_same(&format!("bytes/seed{seed}/{iteration}"), &input);
        }
    }
}

#[test]
fn random_bytes_from_a_small_alphabet() {
    // Biased towards bytes the program actually branches on, which makes deep
    // menu sequences far more likely than with uniform noise.
    const ALPHABET: &[u8] = b"1234567\n \t\0abc/*\"'=<>+-.;{}\xff";
    for seed in [3u64, 8, 64] {
        let mut rng = Rng::new(seed.wrapping_mul(0xA076_1D64_78BD_642F));
        for iteration in 0..60 {
            let n = rng.below(400);
            let mut input = Vec::with_capacity(n);
            for _ in 0..n {
                input.push(*rng.pick(ALPHABET));
            }
            assert_same(&format!("alphabet/seed{seed}/{iteration}"), &input);
        }
    }
}
