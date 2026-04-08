use libfor::for_gen;

fn generate_input(base: u32, length: usize, bits: u32) -> Vec<u32> {
    let max_val = if bits == 0 { 0 } else if bits == 32 { if length == 0 { 0 } else { length as u32 - 1 } } else { (1u32 << bits) - 1 };
    (0..length).map(|i| {
        if bits == 0 { base }
        else if bits == 32 { base + i as u32 }
        else { base + (i as u32 % max_val) }
    }).collect()
}

// Helper: pack with a packN_32 function, unpack with generic_unpack_pub, verify roundtrip
fn verify_pack_unpack_32(bits: u32, pack_fn: fn(u32, &[u32], &mut [u8]) -> u32) {
    let base = 10u32;
    let input = generate_input(base, 32, bits);
    let mut out = vec![0u8; 256];
    let mut tmp = vec![0u32; 32];

    let s1 = pack_fn(base, &input, &mut out);
    let s2 = for_gen::generic_unpack_pub(base, &out, &mut tmp, 32, bits);
    assert_eq!(s1, s2, "pack/unpack size mismatch for bits={}", bits);
    assert_eq!(input, tmp, "data mismatch for bits={}", bits);
}

fn verify_pack_unpack_16(bits: u32, pack_fn: fn(u32, &[u32], &mut [u8]) -> u32) {
    let base = 10u32;
    let input = generate_input(base, 16, bits);
    let mut out = vec![0u8; 256];
    let mut tmp = vec![0u32; 16];

    let s1 = pack_fn(base, &input, &mut out);
    let s2 = for_gen::generic_unpack_pub(base, &out, &mut tmp, 16, bits);
    assert_eq!(s1, s2, "pack16/unpack size mismatch for bits={}", bits);
    assert_eq!(input, tmp, "data16 mismatch for bits={}", bits);
}

fn verify_pack_unpack_8(bits: u32, pack_fn: fn(u32, &[u32], &mut [u8]) -> u32) {
    let base = 10u32;
    let input = generate_input(base, 8, bits);
    let mut out = vec![0u8; 256];
    let mut tmp = vec![0u32; 8];

    let s1 = pack_fn(base, &input, &mut out);
    let s2 = for_gen::generic_unpack_pub(base, &out, &mut tmp, 8, bits);
    assert_eq!(s1, s2, "pack8/unpack size mismatch for bits={}", bits);
    assert_eq!(input, tmp, "data8 mismatch for bits={}", bits);
}

fn verify_pack_unpack_x(bits: u32, pack_fn: fn(u32, &[u32], &mut [u8], u32) -> u32) {
    let base = 10u32;
    for length in 0..8u32 {
        let input = generate_input(base, length as usize, bits);
        let mut out = vec![0u8; 256];
        let mut tmp = vec![0u32; 8];

        let s1 = pack_fn(base, &input, &mut out, length);
        let s2 = for_gen::generic_unpack_x_pub(base, &out, &mut tmp, length, bits);
        assert_eq!(s1, s2, "packx/unpackx size mismatch for bits={} len={}", bits, length);
        assert_eq!(input, tmp[..length as usize], "datax mismatch for bits={} len={}", bits, length);
    }
}

fn verify_linsearch_32(bits: u32, linsearch_fn: fn(u32, &[u8], u32, &mut i32) -> u32) {
    let base = 10u32;
    let input = generate_input(base, 32, bits);
    let mut out = vec![0u8; 256];

    // Pack first using generic
    let mut pack_out = vec![0u8; 256];
    for b in pack_out.iter_mut() { *b = 0; }
    // Use the appropriate pack function via generic
    let nbytes = if bits == 0 { 0u32 } else if bits == 32 { 128 } else { (32 * bits + 7) / 8 };
    // Pack using generic_pack equivalent - we'll use the pack_fn from for_gen
    // Actually, let's just use for_compress_bits from forLib to get packed data
    use libfor::forLib;
    forLib::for_compress_bits(&input, &mut out, 32, base, bits);

    for i in 0..32u32 {
        let mut found: i32 = -1;
        linsearch_fn(base, &out, input[i as usize], &mut found);
        assert!(found >= 0, "linsearch32 not found for bits={} i={}", bits, i);
        assert_eq!(input[found as usize], input[i as usize],
            "linsearch32 wrong result for bits={} i={}", bits, i);
    }
}

fn verify_linsearch_x(bits: u32, linsearch_fn: fn(u32, &[u8], u32, u32, &mut i32) -> u32) {
    let base = 10u32;
    for length in 1..8u32 {
        let input = generate_input(base, length as usize, bits);
        let mut out = vec![0u8; 256];
        use libfor::forLib;
        forLib::for_compress_bits(&input, &mut out, length, base, bits);

        for i in 0..length {
            let mut found: i32 = -1;
            linsearch_fn(base, &out, length, input[i as usize], &mut found);
            assert!(found >= 0, "linsearchx not found for bits={} len={} i={}", bits, length, i);
            assert_eq!(input[found as usize], input[i as usize]);
        }
    }
}

#[test]
fn test_pack_unpack_32_all_bits() {
    let fns: Vec<(u32, fn(u32, &[u32], &mut [u8]) -> u32)> = vec![
        (0, for_gen::pack0_32), (1, for_gen::pack1_32), (2, for_gen::pack2_32),
        (3, for_gen::pack3_32), (4, for_gen::pack4_32), (5, for_gen::pack5_32),
        (6, for_gen::pack6_32), (7, for_gen::pack7_32), (8, for_gen::pack8_32),
        (9, for_gen::pack9_32), (10, for_gen::pack10_32), (11, for_gen::pack11_32),
        (12, for_gen::pack12_32), (13, for_gen::pack13_32), (14, for_gen::pack14_32),
        (15, for_gen::pack15_32), (16, for_gen::pack16_32), (17, for_gen::pack17_32),
        (18, for_gen::pack18_32), (19, for_gen::pack19_32), (20, for_gen::pack20_32),
        (21, for_gen::pack21_32), (22, for_gen::pack22_32), (23, for_gen::pack23_32),
        (24, for_gen::pack24_32), (25, for_gen::pack25_32), (26, for_gen::pack26_32),
        (27, for_gen::pack27_32), (28, for_gen::pack28_32), (29, for_gen::pack29_32),
        (30, for_gen::pack30_32), (31, for_gen::pack31_32), (32, for_gen::pack32_32),
    ];
    for (bits, f) in fns { verify_pack_unpack_32(bits, f); }
}

#[test]
fn test_pack_unpack_16_all_bits() {
    let fns: Vec<(u32, fn(u32, &[u32], &mut [u8]) -> u32)> = vec![
        (0, for_gen::pack0_16), (1, for_gen::pack1_16), (2, for_gen::pack2_16),
        (3, for_gen::pack3_16), (4, for_gen::pack4_16), (5, for_gen::pack5_16),
        (6, for_gen::pack6_16), (7, for_gen::pack7_16), (8, for_gen::pack8_16),
        (9, for_gen::pack9_16), (10, for_gen::pack10_16), (11, for_gen::pack11_16),
        (12, for_gen::pack12_16), (13, for_gen::pack13_16), (14, for_gen::pack14_16),
        (15, for_gen::pack15_16), (16, for_gen::pack16_16), (17, for_gen::pack17_16),
        (18, for_gen::pack18_16), (19, for_gen::pack19_16), (20, for_gen::pack20_16),
        (21, for_gen::pack21_16), (22, for_gen::pack22_16), (23, for_gen::pack23_16),
        (24, for_gen::pack24_16), (25, for_gen::pack25_16), (26, for_gen::pack26_16),
        (27, for_gen::pack27_16), (28, for_gen::pack28_16), (29, for_gen::pack29_16),
        (30, for_gen::pack30_16), (31, for_gen::pack31_16), (32, for_gen::pack32_16),
    ];
    for (bits, f) in fns { verify_pack_unpack_16(bits, f); }
}

#[test]
fn test_pack_unpack_8_all_bits() {
    let fns: Vec<(u32, fn(u32, &[u32], &mut [u8]) -> u32)> = vec![
        (0, for_gen::pack0_8), (1, for_gen::pack1_8), (2, for_gen::pack2_8),
        (3, for_gen::pack3_8), (4, for_gen::pack4_8), (5, for_gen::pack5_8),
        (6, for_gen::pack6_8), (7, for_gen::pack7_8), (8, for_gen::pack8_8),
        (9, for_gen::pack9_8), (10, for_gen::pack10_8), (11, for_gen::pack11_8),
        (12, for_gen::pack12_8), (13, for_gen::pack13_8), (14, for_gen::pack14_8),
        (15, for_gen::pack15_8), (16, for_gen::pack16_8), (17, for_gen::pack17_8),
        (18, for_gen::pack18_8), (19, for_gen::pack19_8), (20, for_gen::pack20_8),
        (21, for_gen::pack21_8), (22, for_gen::pack22_8), (23, for_gen::pack23_8),
        (24, for_gen::pack24_8), (25, for_gen::pack25_8), (26, for_gen::pack26_8),
        (27, for_gen::pack27_8), (28, for_gen::pack28_8), (29, for_gen::pack29_8),
        (30, for_gen::pack30_8), (31, for_gen::pack31_8), (32, for_gen::pack32_8),
    ];
    for (bits, f) in fns { verify_pack_unpack_8(bits, f); }
}

#[test]
fn test_pack_unpack_x_all_bits() {
    let fns: Vec<(u32, fn(u32, &[u32], &mut [u8], u32) -> u32)> = vec![
        (0, for_gen::pack0_x), (1, for_gen::pack1_x), (2, for_gen::pack2_x),
        (3, for_gen::pack3_x), (4, for_gen::pack4_x), (5, for_gen::pack5_x),
        (6, for_gen::pack6_x), (7, for_gen::pack7_x), (8, for_gen::pack8_x),
        (9, for_gen::pack9_x), (10, for_gen::pack10_x), (11, for_gen::pack11_x),
        (12, for_gen::pack12_x), (13, for_gen::pack13_x), (14, for_gen::pack14_x),
        (15, for_gen::pack15_x), (16, for_gen::pack16_x), (17, for_gen::pack17_x),
        (18, for_gen::pack18_x), (19, for_gen::pack19_x), (20, for_gen::pack20_x),
        (21, for_gen::pack21_x), (22, for_gen::pack22_x), (23, for_gen::pack23_x),
        (24, for_gen::pack24_x), (25, for_gen::pack25_x), (26, for_gen::pack26_x),
        (27, for_gen::pack27_x), (28, for_gen::pack28_x), (29, for_gen::pack29_x),
        (30, for_gen::pack30_x), (31, for_gen::pack31_x), (32, for_gen::pack32_x),
    ];
    for (bits, f) in fns { verify_pack_unpack_x(bits, f); }
}

#[test]
fn test_linsearch_32_selected_bits() {
    let fns: Vec<(u32, fn(u32, &[u8], u32, &mut i32) -> u32)> = vec![
        (0, for_gen::linsearch0_32), (1, for_gen::linsearch1_32),
        (5, for_gen::linsearch5_32), (8, for_gen::linsearch8_32),
        (16, for_gen::linsearch16_32), (32, for_gen::linsearch32_32),
    ];
    for (bits, f) in fns { verify_linsearch_32(bits, f); }
}

#[test]
fn test_linsearch_x_selected_bits() {
    let fns: Vec<(u32, fn(u32, &[u8], u32, u32, &mut i32) -> u32)> = vec![
        (1, for_gen::linsearch1_x), (5, for_gen::linsearch5_x),
        (8, for_gen::linsearch8_x), (16, for_gen::linsearch16_x),
        (32, for_gen::linsearch32_x),
    ];
    for (bits, f) in fns { verify_linsearch_x(bits, f); }
}

#[test]
fn test_generic_unpack_pub_specific_values() {
    // Verify generic_unpack_pub with known packed data from forLib
    use libfor::forLib;
    let base = 10u32;
    let bits = 5u32;
    let input: Vec<u32> = (0..32).map(|i| base + (i % 31)).collect();
    let mut packed = vec![0u8; 256];
    forLib::for_compress_bits(&input, &mut packed, 32, base, bits);

    let mut output = vec![0u32; 32];
    let consumed = for_gen::generic_unpack_pub(base, &packed, &mut output, 32, bits);
    assert_eq!(consumed, 20); // 32*5/8 = 20
    assert_eq!(output, input);
}

#[test]
fn test_generic_linsearch_pub() {
    use libfor::forLib;
    let base = 10u32;
    let bits = 8u32;
    let input: Vec<u32> = (0..32).map(|i| base + (i % 255)).collect();
    let mut packed = vec![0u8; 256];
    forLib::for_compress_bits(&input, &mut packed, 32, base, bits);

    // Search for existing value
    let mut found: i32 = -1;
    for_gen::generic_linsearch_pub(base, &packed, 32, bits, input[5], &mut found);
    assert_eq!(found, 5);

    // Search for non-existing value
    found = -1;
    for_gen::generic_linsearch_pub(base, &packed, 32, bits, base + 255 + 1, &mut found);
    assert_eq!(found, -1);
}

#[test]
fn test_generic_linsearch_x_pub() {
    use libfor::forLib;
    let base = 10u32;
    let bits = 5u32;
    let input: Vec<u32> = (0..5).map(|i| base + (i % 31)).collect();
    let mut packed = vec![0u8; 256];
    forLib::for_compress_bits(&input, &mut packed, 5, base, bits);

    let mut found: i32 = -1;
    for_gen::generic_linsearch_x_pub(base, &packed, 5, bits, input[3], &mut found);
    assert_eq!(found, 3);

    found = -1;
    for_gen::generic_linsearch_x_pub(base, &packed, 5, bits, 999, &mut found);
    assert_eq!(found, -1);
}

#[test]
fn test_pack0_returns_zero() {
    let input = vec![10u32; 32];
    let mut out = vec![0u8; 256];
    assert_eq!(for_gen::pack0_32(10, &input, &mut out), 0);
    assert_eq!(for_gen::pack0_16(10, &input[..16], &mut out), 0);
    assert_eq!(for_gen::pack0_8(10, &input[..8], &mut out), 0);
    assert_eq!(for_gen::pack0_x(10, &input[..5], &mut out, 5), 0);
}

#[test]
fn test_unpack0_fills_base() {
    let input = vec![0u8; 256];
    let mut out32 = vec![0u32; 32];
    let mut out16 = vec![0u32; 16];
    let mut out8 = vec![0u32; 8];
    let mut outx = vec![0u32; 5];

    assert_eq!(for_gen::unpack0_32(42, &input, &mut out32), 0);
    assert!(out32.iter().all(|&v| v == 42));

    assert_eq!(for_gen::unpack0_16(42, &input, &mut out16), 0);
    assert!(out16.iter().all(|&v| v == 42));

    assert_eq!(for_gen::unpack0_8(42, &input, &mut out8), 0);
    assert!(out8.iter().all(|&v| v == 42));

    assert_eq!(for_gen::unpack0_x(42, &input, &mut outx, 5), 0);
    assert!(outx.iter().all(|&v| v == 42));
}

#[test]
fn test_linsearch0_finds_base() {
    let input = vec![0u8; 256];
    let mut found: i32 = -1;
    for_gen::linsearch0_32(10, &input, 10, &mut found);
    assert_eq!(found, 0);

    found = -1;
    for_gen::linsearch0_32(10, &input, 11, &mut found);
    assert_eq!(found, -1);

    found = -1;
    for_gen::linsearch0_16(10, &input, 10, &mut found);
    assert_eq!(found, 0);

    found = -1;
    for_gen::linsearch0_8(10, &input, 10, &mut found);
    assert_eq!(found, 0);

    found = -1;
    for_gen::linsearch0_x(10, &input, 3, 10, &mut found);
    assert_eq!(found, 0);

    found = -1;
    for_gen::linsearch0_x(10, &input, 0, 10, &mut found);
    assert_eq!(found, -1);
}

fn main() {}
