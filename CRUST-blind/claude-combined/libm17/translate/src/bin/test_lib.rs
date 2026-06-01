use libm17::types::{FrameType, PreambleType, LSF, SYM_PER_FRA, SYM_PER_PLD, SYM_PER_SWD};
use libm17::{send_data, send_eot, send_frame, send_preamble, send_syncword};

#[test]
fn test_send_preamble_lsf() {
    let mut buf: [f32; SYM_PER_FRA] = [0.0; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_preamble(&mut buf, &mut cnt, PreambleType::Lsf);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    assert_eq!(buf[0], 3.0);
    assert_eq!(buf[1], -3.0);
    assert_eq!(buf[2], 3.0);
    assert_eq!(buf[3], -3.0);
    assert_eq!(buf[188], 3.0);
    assert_eq!(buf[189], -3.0);
    assert_eq!(buf[190], 3.0);
    assert_eq!(buf[191], -3.0);
}

#[test]
fn test_send_preamble_bert() {
    let mut buf: [f32; SYM_PER_FRA] = [0.0; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_preamble(&mut buf, &mut cnt, PreambleType::Bert);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    assert_eq!(buf[0], -3.0);
    assert_eq!(buf[1], 3.0);
    assert_eq!(buf[2], -3.0);
    assert_eq!(buf[3], 3.0);
}

#[test]
fn test_send_eot() {
    let mut buf: [f32; SYM_PER_FRA] = [0.0; SYM_PER_FRA];
    let mut cnt: u32 = 0;
    send_eot(&mut buf, &mut cnt);
    assert_eq!(cnt, SYM_PER_FRA as u32);
    let pattern = [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0];
    for i in 0..SYM_PER_FRA {
        assert_eq!(buf[i], pattern[i % 8]);
    }
}

#[test]
fn test_send_syncword() {
    let mut swbuf: [f32; SYM_PER_SWD] = [0.0; SYM_PER_SWD];
    let mut cnt: u32 = 0;
    send_syncword(&mut swbuf, &mut cnt, 0x55F7);
    assert_eq!(cnt, SYM_PER_SWD as u32);
    // From C: 3 3 3 3 -3 -3 3 -3
    assert_eq!(swbuf, [3.0, 3.0, 3.0, 3.0, -3.0, -3.0, 3.0, -3.0]);
}

#[test]
fn test_send_data() {
    let mut dbuf: [f32; SYM_PER_PLD] = [0.0; SYM_PER_PLD];
    let mut cnt: u32 = 0;
    let mut data: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    for i in 0..(SYM_PER_PLD * 2) {
        data[i] = (i & 1) as u8;
    }
    send_data(&mut dbuf, &mut cnt, &data);
    assert_eq!(cnt, SYM_PER_PLD as u32);
    // For dibits (0,1) -> idx=1 -> SYMBOL_MAP[1]=3
    for i in 0..SYM_PER_PLD {
        assert_eq!(dbuf[i], 3.0);
    }
}

#[test]
fn test_send_data_zeros() {
    let mut dbuf: [f32; SYM_PER_PLD] = [0.0; SYM_PER_PLD];
    let mut cnt: u32 = 0;
    let data: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    send_data(&mut dbuf, &mut cnt, &data);
    // For dibits (0,0) -> idx=0 -> SYMBOL_MAP[0]=1
    for i in 0..SYM_PER_PLD {
        assert_eq!(dbuf[i], 1.0);
    }
}

#[test]
fn test_send_frame_lsf_starts_with_syncword() {
    let mut lsf = LSF::default();
    for i in 0..6u8 {
        lsf.dst[i as usize] = i + 1;
        lsf.src[i as usize] = 0x10 + i;
    }
    lsf.type_field[0] = 0xAB;
    lsf.type_field[1] = 0xCD;
    for i in 0..14u8 {
        lsf.meta[i as usize] = i * 3;
    }
    lsf.crc[0] = 0x01;
    lsf.crc[1] = 0x74;

    let mut buf: [f32; SYM_PER_FRA] = [0.0; SYM_PER_FRA];
    let dummy: [u8; 16] = [0; 16];
    send_frame(&mut buf, &dummy, FrameType::Lsf, &lsf, 0, 0);

    // First 8 symbols should be the LSF syncword: 3 3 3 3 -3 -3 3 -3
    assert_eq!(buf[0], 3.0);
    assert_eq!(buf[1], 3.0);
    assert_eq!(buf[2], 3.0);
    assert_eq!(buf[3], 3.0);
    assert_eq!(buf[4], -3.0);
    assert_eq!(buf[5], -3.0);
    assert_eq!(buf[6], 3.0);
    assert_eq!(buf[7], -3.0);

    // Validated against C output for the first payload symbols
    assert_eq!(buf[8], -1.0);
    assert_eq!(buf[9], 3.0);
    assert_eq!(buf[10], 1.0);
    assert_eq!(buf[11], -1.0);
}

fn main() {}
