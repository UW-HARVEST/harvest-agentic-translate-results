use Megalania::probability::{Prob, NUM_BIT_MODEL_TOTAL_BITS, PROB_INIT_VAL};

#[test]
fn test_constants() {
    assert_eq!(NUM_BIT_MODEL_TOTAL_BITS, 11);
    assert_eq!(PROB_INIT_VAL, 1024);
}

#[test]
fn test_prob_type() {
    let p: Prob = 1024;
    assert_eq!(p, 1024u16);
}

fn main() {}
