use libm17::types::*;

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
    assert_eq!(SW_LEN, 10 * 8);
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
fn test_frame_type_eq() {
    assert_eq!(FrameType::Lsf, FrameType::Lsf);
    assert_ne!(FrameType::Lsf, FrameType::Str);
    assert_ne!(FrameType::Str, FrameType::Pkt);
}

#[test]
fn test_preamble_type_eq() {
    assert_eq!(PreambleType::Lsf, PreambleType::Lsf);
    assert_ne!(PreambleType::Lsf, PreambleType::Bert);
}

fn main() {}
