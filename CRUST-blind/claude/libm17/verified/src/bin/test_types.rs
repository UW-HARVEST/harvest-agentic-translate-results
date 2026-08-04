use libm17::types::{
    FrameType, PreambleType, BSB_SPS, CHAR_MAP, FLT_SPAN, LSF, NUM_STATES, RRC_DEV, SYM_PER_FRA,
    SYM_PER_PLD, SYM_PER_SWD, SW_LEN, U40_9, U40_9_8,
};

#[test]
fn test_constants() {
    assert_eq!(SYM_PER_SWD, 8);
    assert_eq!(FLT_SPAN, 8);
    assert_eq!(RRC_DEV, 7168.0);
    assert_eq!(SYM_PER_PLD, 184);
    assert_eq!(SYM_PER_FRA, 192);
    assert_eq!(NUM_STATES, 16);
    assert_eq!(CHAR_MAP, " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/.");
    assert_eq!(U40_9, 262144000000000u64);
    assert_eq!(U40_9_8, 268697600000000u64);
    assert_eq!(BSB_SPS, 10);
    assert_eq!(SW_LEN, BSB_SPS * SYM_PER_SWD);
    // Verify CHAR_MAP length
    assert_eq!(CHAR_MAP.len(), 40);
    // Verify U40_9 = 40^9
    assert_eq!(U40_9, 40u64.pow(9));
    // Verify U40_9_8 = 40^9 + 40^8
    assert_eq!(U40_9_8, 40u64.pow(9) + 40u64.pow(8));
}

#[test]
fn test_lsf_default() {
    let lsf = LSF::default();
    assert_eq!(lsf.dst, [0u8; 6]);
    assert_eq!(lsf.src, [0u8; 6]);
    assert_eq!(lsf.type_field, [0u8; 2]);
    assert_eq!(lsf.meta, [0u8; 14]);
    assert_eq!(lsf.crc, [0u8; 2]);
}

#[test]
fn test_lsf_clone() {
    let mut lsf = LSF::default();
    for i in 0..6 {
        lsf.dst[i] = i as u8 + 1;
    }
    let lsf2 = lsf.clone();
    assert_eq!(lsf2.dst, lsf.dst);
}

#[test]
fn test_frame_type_eq() {
    assert_eq!(FrameType::Lsf, FrameType::Lsf);
    assert_ne!(FrameType::Lsf, FrameType::Str);
    assert_ne!(FrameType::Lsf, FrameType::Pkt);
    assert_ne!(FrameType::Str, FrameType::Pkt);
}

#[test]
fn test_preamble_type_eq() {
    assert_eq!(PreambleType::Lsf, PreambleType::Lsf);
    assert_ne!(PreambleType::Lsf, PreambleType::Bert);
}

fn main() {}
