//! Phase B — valid-path differential tests for `get_os_arch`.
//! Covers CONFIGS.md rows 1-6.

mod common;
use common::*;

/// Filler bytes that can never accidentally form an arch name.
const FILLER: &[u8] = b"QWZYKJHGFDVN-_/. 0123456789";

fn random_filler(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| *rng.pick(FILLER)).collect()
}

/// Random filler with a random length in `lo..=hi`.
fn filler(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    random_filler(rng, n)
}

/// Row 1: each of the 12 table entries alone, at a random position inside
/// random filler.
#[test]
fn row01_each_arch_at_random_position() {
    let mut rng = Rng::new(0x1001);
    for (idx, arch) in ARCHS.iter().enumerate() {
        for iter in 0..400 {
            let pre = filler(&mut rng, 0, 40);
            let post = filler(&mut rng, 0, 40);
            let mut s = Vec::new();
            s.extend_from_slice(&pre);
            s.extend_from_slice(arch.as_bytes());
            s.extend_from_slice(&post);
            diff_get_os_arch(&format!("row1 arch#{idx}={arch} iter={iter}"), &s);
        }
    }
}

/// Row 2: arch at the very start, the very end, and as the entire string.
#[test]
fn row02_arch_at_boundaries() {
    let mut rng = Rng::new(0x1002);
    for arch in ARCHS.iter() {
        let a = arch.as_bytes();
        // whole string
        diff_get_os_arch("row2 whole", a);
        for _ in 0..100 {
            let tail = filler(&mut rng, 1, 30);
            let head = filler(&mut rng, 1, 30);

            let mut start = a.to_vec();
            start.extend_from_slice(&tail);
            diff_get_os_arch("row2 start", &start);

            let mut end = head.clone();
            end.extend_from_slice(a);
            diff_get_os_arch("row2 end", &end);
        }
    }
}

/// Row 3: two archs present — the *table order* must win, never the string
/// position. All 132 ordered pairs, in both string orders.
#[test]
fn row03_table_order_beats_string_position() {
    let mut rng = Rng::new(0x1003);
    for (i, a) in ARCHS.iter().enumerate() {
        for (j, b) in ARCHS.iter().enumerate() {
            if i == j {
                continue;
            }
            for _ in 0..8 {
                let sep = filler(&mut rng, 0, 8);
                let mut s = Vec::new();
                s.extend_from_slice(a.as_bytes());
                s.extend_from_slice(&sep);
                s.extend_from_slice(b.as_bytes());
                diff_get_os_arch(&format!("row3 {a}..{b}"), &s);

                let mut t = Vec::new();
                t.extend_from_slice(b.as_bytes());
                t.extend_from_slice(&sep);
                t.extend_from_slice(a.as_bytes());
                diff_get_os_arch(&format!("row3 {b}..{a}"), &t);
            }
        }
    }
}

/// Row 3b: three or more archs at once, random subsets and orders.
#[test]
fn row03b_many_archs_at_once() {
    let mut rng = Rng::new(0x1003b);
    for _ in 0..4000 {
        let n = rng.range(3, 12);
        let mut s = filler(&mut rng, 0, 10);
        for _ in 0..n {
            let arch = *rng.pick(&ARCHS);
            s.extend_from_slice(arch.as_bytes());
            let gap = filler(&mut rng, 0, 5);
            s.extend_from_slice(&gap);
        }
        diff_get_os_arch("row3b", &s);
    }
}

/// Row 4: deliberately confusable / overlapping pairs.
#[test]
fn row04_overlapping_confusable_pairs() {
    let cases: &[&[u8]] = &[
        b"i386 i686",
        b"i686 i386",
        b"amd64 ia64",
        b"ia64 amd64",
        b"aarch64 arm64",
        b"arm64 aarch64",
        b"armv6 armv7",
        b"armv7 armv6",
        b"x86_64 amd64",
        b"amd64 x86_64",
        // `aarch64` textually contains `arch64`; `arm64` is not a substring of it
        b"aarch64",
        b"arm64",
        // `i86pc` vs `i386`/`i686`
        b"i86pc",
        b"i86pc i386",
        b"i386 i86pc",
        // an arch embedded inside a longer token
        b"myi386arch",
        b"prefixx86_64suffix",
        b"AAIXX",
        b"xamd64x",
        b"sparcv9",
        b"sun4v sparc SUNW",
        // digits/dots adjacent
        b"1i6862",
        b"ia64.0",
        b"..arm64..",
    ];
    for c in cases {
        diff_get_os_arch("row4", c);
    }
}

/// Row 5: random filler with no arch, lengths 0..4096.
#[test]
fn row05_no_arch_random_lengths() {
    let mut rng = Rng::new(0x1005);
    diff_get_os_arch("row5 empty", b"");
    for _ in 0..3000 {
        let s = filler(&mut rng, 0, 200);
        diff_get_os_arch("row5 short", &s);
    }
    for _ in 0..40 {
        let s = filler(&mut rng, 3000, 4096);
        diff_get_os_arch("row5 long", &s);
    }
}

/// Row 5b: long strings *with* an arch at a random offset (up to 4 KiB).
#[test]
fn row05b_arch_in_long_string() {
    let mut rng = Rng::new(0x1005b);
    for _ in 0..300 {
        let total = rng.range(1000, 4096);
        let arch = *rng.pick(&ARCHS);
        let a = arch.as_bytes();
        let at = rng.below(total.saturating_sub(a.len()).max(1));
        let mut s = random_filler(&mut rng, total);
        if at + a.len() <= s.len() {
            s[at..at + a.len()].copy_from_slice(a);
        }
        diff_get_os_arch("row5b", &s);
    }
}

/// Row 6: only a *prefix* (or suffix) of an arch name appears.
#[test]
fn row06_arch_prefixes_only() {
    let mut rng = Rng::new(0x1006);
    for arch in ARCHS.iter() {
        let a = arch.as_bytes();
        for cut in 1..a.len() {
            let mut s = filler(&mut rng, 0, 12);
            s.extend_from_slice(&a[..cut]);
            let tail = filler(&mut rng, 0, 12);
            s.extend_from_slice(&tail);
            diff_get_os_arch(&format!("row6 prefix {arch}[..{cut}]"), &s);

            let mut t = filler(&mut rng, 0, 12);
            t.extend_from_slice(&a[cut..]);
            let tail2 = filler(&mut rng, 0, 12);
            t.extend_from_slice(&tail2);
            diff_get_os_arch(&format!("row6 suffix {arch}[{cut}..]"), &t);
        }
    }
}

/// Row 6b: byte-level fuzz over an alphabet biased towards arch characters, so
/// arch names appear by chance and every `strstr` outcome is hit.
#[test]
fn row06b_biased_byte_fuzz() {
    const ALPHA: &[u8] = b"xX86_64iI3prc68aAmdIsparcv71hbnv6yz.|[]: ";
    let mut rng = Rng::new(0x1006b);
    for _ in 0..20000 {
        let len = rng.range(0, 48);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHA)).collect();
        diff_get_os_arch("row6b", &s);
    }
}
