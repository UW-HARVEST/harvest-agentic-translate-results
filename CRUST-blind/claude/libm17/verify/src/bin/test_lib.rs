use libm17::types::{
    FrameType, PreambleType, LSF, SYM_PER_FRA, SYM_PER_PLD, SYM_PER_SWD,
};
use libm17::{send_data, send_eot, send_frame, send_preamble, send_syncword};

#[test]
fn test_send_preamble_lsf() {
    let mut buf = [0f32; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_preamble(&mut buf, &mut cnt, PreambleType::Lsf);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    // Expected first 8 from C: 3 -3 3 -3 3 -3 3 -3
    let expected = [3.0f32, -3.0, 3.0, -3.0, 3.0, -3.0, 3.0, -3.0];
    for i in 0..8 {
        assert_eq!(buf[i], expected[i]);
    }
    // Verify whole pattern
    for i in 0..SYM_PER_FRA {
        let expected_v = if i % 2 == 0 { 3.0 } else { -3.0 };
        assert_eq!(buf[i], expected_v);
    }
}

#[test]
fn test_send_preamble_bert() {
    let mut buf = [0f32; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_preamble(&mut buf, &mut cnt, PreambleType::Bert);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    let expected = [-3.0f32, 3.0, -3.0, 3.0, -3.0, 3.0, -3.0, 3.0];
    for i in 0..8 {
        assert_eq!(buf[i], expected[i]);
    }
    for i in 0..SYM_PER_FRA {
        let expected_v = if i % 2 == 0 { -3.0 } else { 3.0 };
        assert_eq!(buf[i], expected_v);
    }
}

#[test]
fn test_send_eot() {
    let mut buf = [0f32; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_eot(&mut buf, &mut cnt);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    // Pattern: [3,3,3,3,3,3,-3,3] repeats
    let expected_pat = [3.0f32, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0];
    for i in 0..16 {
        assert_eq!(buf[i], expected_pat[i % 8]);
    }
    for i in 0..SYM_PER_FRA {
        assert_eq!(buf[i], expected_pat[i % 8]);
    }
}

#[test]
fn test_send_data() {
    let mut buf = [0f32; SYM_PER_PLD];
    let mut cnt: u32 = 0;
    let mut input = [0u8; 368];
    for i in 0..368 {
        input[i] = if i % 4 < 2 { 0 } else { 1 };
    }
    send_data(&mut buf, &mut cnt, &input);
    assert_eq!(cnt, SYM_PER_PLD as u32);
    // For input pattern (0,0,1,1,0,0,1,1,...), dibits are (0,0)=0->1 and (1,1)=3->-3
    let expected = [1.0f32, -3.0, 1.0, -3.0, 1.0, -3.0, 1.0, -3.0];
    for i in 0..8 {
        assert_eq!(buf[i], expected[i]);
    }
}

#[test]
fn test_send_syncword_lsf() {
    let mut buf = [0f32; SYM_PER_SWD];
    let mut cnt: u32 = 0;
    send_syncword(&mut buf, &mut cnt, 0x55F7);
    assert_eq!(cnt, SYM_PER_SWD as u32);
    // Expected: 3 3 3 3 -3 -3 3 -3
    let expected = [3.0f32, 3.0, 3.0, 3.0, -3.0, -3.0, 3.0, -3.0];
    assert_eq!(buf, expected);
}

#[test]
fn test_send_syncword_str() {
    let mut buf = [0f32; SYM_PER_SWD];
    let mut cnt: u32 = 0;
    send_syncword(&mut buf, &mut cnt, 0xFF5D);
    let expected = [-3.0f32, -3.0, -3.0, -3.0, 3.0, 3.0, -3.0, 3.0];
    assert_eq!(buf, expected);
}

#[test]
fn test_send_syncword_pkt() {
    let mut buf = [0f32; SYM_PER_SWD];
    let mut cnt: u32 = 0;
    send_syncword(&mut buf, &mut cnt, 0x75FF);
    let expected = [3.0f32, -3.0, 3.0, 3.0, -3.0, -3.0, -3.0, -3.0];
    assert_eq!(buf, expected);
}

fn build_test_lsf() -> LSF {
    let mut lsf = LSF::default();
    for i in 0..6 {
        lsf.dst[i] = 0x10 + i as u8;
    }
    for i in 0..6 {
        lsf.src[i] = 0x20 + i as u8;
    }
    lsf.type_field[0] = 0xAA;
    lsf.type_field[1] = 0xBB;
    for i in 0..14 {
        lsf.meta[i] = 0x40 + i as u8;
    }
    lsf.crc[0] = 0x12;
    lsf.crc[1] = 0x34;
    lsf
}

#[test]
fn test_send_frame_lsf() {
    let lsf = build_test_lsf();
    let mut buf = [0f32; SYM_PER_FRA];
    let dummy = [0u8; 16];
    send_frame(&mut buf, &dummy, FrameType::Lsf, &lsf, 0, 0);
    // First 8 = sync (3 3 3 3 -3 -3 3 -3)
    let sync_expected = [3.0f32, 3.0, 3.0, 3.0, -3.0, -3.0, 3.0, -3.0];
    for i in 0..8 {
        assert_eq!(buf[i], sync_expected[i]);
    }
    // From C: bytes 8..16 = -3 1 3 3 3 3 1 -1
    let next8 = [-3.0f32, 1.0, 3.0, 3.0, 3.0, 3.0, 1.0, -1.0];
    for i in 0..8 {
        assert_eq!(buf[8 + i], next8[i]);
    }
    // bytes 16..32 = -3 3 -3 1 1 -1 -3 -1 -1 -1 3 -3 -1 1 -3 -3
    let next16 = [
        -3.0f32, 3.0, -3.0, 1.0, 1.0, -1.0, -3.0, -1.0, -1.0, -1.0, 3.0, -3.0, -1.0, 1.0, -3.0,
        -3.0,
    ];
    for i in 0..16 {
        assert_eq!(buf[16 + i], next16[i]);
    }
    // Sum-of-squares check (overall stability)
    let sumsq: f32 = buf.iter().map(|&v| v * v).sum();
    assert!((sumsq - 992.0).abs() < 0.5);
}

#[test]
fn test_send_frame_str() {
    let lsf = build_test_lsf();
    let mut buf = [0f32; SYM_PER_FRA];
    let mut data = [0u8; 16];
    for i in 0..16 {
        data[i] = i as u8;
    }
    send_frame(&mut buf, &data, FrameType::Str, &lsf, 0, 1);
    // First 8 = STR sync = -3 -3 -3 -3 3 3 -3 3
    let sync_expected = [-3.0f32, -3.0, -3.0, -3.0, 3.0, 3.0, -3.0, 3.0];
    for i in 0..8 {
        assert_eq!(buf[i], sync_expected[i]);
    }
    // bytes 8..16 from C: -3 -3 1 3 -3 -1 -1 1
    let next8 = [-3.0f32, -3.0, 1.0, 3.0, -3.0, -1.0, -1.0, 1.0];
    for i in 0..8 {
        assert_eq!(buf[8 + i], next8[i]);
    }
    let sumsq: f32 = buf.iter().map(|&v| v * v).sum();
    assert!((sumsq - 1048.0).abs() < 0.5);
}

#[test]
fn test_send_frame_pkt() {
    let lsf = build_test_lsf(); // not used for PKT but needed by API
    let mut buf = [0f32; SYM_PER_FRA];
    let mut data = [0u8; 26];
    for i in 0..26 {
        data[i] = i as u8;
    }
    send_frame(&mut buf, &data, FrameType::Pkt, &lsf, 0, 0);
    // First 8 = PKT sync = 3 -3 3 3 -3 -3 -3 -3
    let sync_expected = [3.0f32, -3.0, 3.0, 3.0, -3.0, -3.0, -3.0, -3.0];
    for i in 0..8 {
        assert_eq!(buf[i], sync_expected[i]);
    }
    // bytes 8..16 from C: -3 -3 1 3 -1 1 1 -1
    let next8 = [-3.0f32, -3.0, 1.0, 3.0, -1.0, 1.0, 1.0, -1.0];
    for i in 0..8 {
        assert_eq!(buf[8 + i], next8[i]);
    }
    let sumsq: f32 = buf.iter().map(|&v| v * v).sum();
    assert!((sumsq - 1056.0).abs() < 0.5);
}

fn main() {}
