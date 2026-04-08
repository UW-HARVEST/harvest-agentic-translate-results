use libm17::types::*;

#[test]
fn test_type_constants() {
    assert_eq!(SYM_PER_SWD, 8);
    assert_eq!(FLT_SPAN, 8);
    assert_eq!(RRC_DEV, 7168.0);
    assert_eq!(SYM_PER_PLD, 184);
    assert_eq!(SYM_PER_FRA, 192);
    assert_eq!(NUM_STATES, 16);
    assert_eq!(CHAR_MAP, " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/.");
    assert_eq!(BSB_SPS, 10);
    assert_eq!(SW_LEN, 80);
}

#[test]
fn test_frame_type_enum() {
    let lsf = FrameType::Lsf;
    let str_ = FrameType::Str;
    let pkt = FrameType::Pkt;
    assert_ne!(lsf, str_);
    assert_ne!(str_, pkt);
    assert_ne!(lsf, pkt);
    assert_eq!(lsf, FrameType::Lsf);
}

#[test]
fn test_preamble_type_enum() {
    assert_eq!(PreambleType::Lsf, PreambleType::Lsf);
    assert_ne!(PreambleType::Lsf, PreambleType::Bert);
}

#[test]
fn test_lsf_default() {
    let lsf = LSF::default();
    assert_eq!(lsf.dst, [0; 6]);
    assert_eq!(lsf.src, [0; 6]);
    assert_eq!(lsf.type_field, [0; 2]);
    assert_eq!(lsf.meta, [0; 14]);
    assert_eq!(lsf.crc, [0; 2]);
}

fn main() {}
