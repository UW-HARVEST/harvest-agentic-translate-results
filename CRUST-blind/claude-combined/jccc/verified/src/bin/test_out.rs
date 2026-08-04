use jccc::out::{
    CODE_BLUE, CODE_CYAN, CODE_GREEN, CODE_MAGENTA, CODE_RED, CODE_WHITE, CODE_YELLOW, OUTPUT_STREAM,
};

#[test]
fn test_color_codes() {
    assert_eq!(CODE_RED, 31);
    assert_eq!(CODE_YELLOW, 33);
    assert_eq!(CODE_GREEN, 32);
    assert_eq!(CODE_BLUE, 34);
    assert_eq!(CODE_MAGENTA, 35);
    assert_eq!(CODE_CYAN, 36);
    assert_eq!(CODE_WHITE, 37);
}

#[test]
fn test_output_stream_constant() {
    assert_eq!(OUTPUT_STREAM, "stderr");
}

fn main() {}
