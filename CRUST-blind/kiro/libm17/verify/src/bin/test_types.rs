use libm17::types::*;

#[test]
fn test_frame_type_variants() {
    let _lsf = FrameType::Lsf;
    let _str = FrameType::Str;
    let _pkt = FrameType::Pkt;
}

#[test]
fn test_preamble_type_variants() {
    let _lsf = PreambleType::Lsf;
    let _bert = PreambleType::Bert;
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

#[test]
fn test_constants() {
    assert_eq!(SYM_PER_SWD, 8);
    assert_eq!(SYM_PER_PLD, 184);
    assert_eq!(SYM_PER_FRA, 192);
    assert_eq!(BSB_SPS, 10);
    assert_eq!(SW_LEN, 80);
    assert_eq!(FLT_SPAN, 8);
    assert_eq!(NUM_STATES, 16);
    assert_eq!(CHAR_MAP, " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/.");
    assert_eq!(U40_9, 262144000000000);
    assert_eq!(U40_9_8, 268697600000000);
}

fn main() {}
