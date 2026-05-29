use Megalania::probability::{Prob, NUM_BIT_MODEL_TOTAL_BITS, PROB_INIT_VAL};

#[test]
fn test_constants() {
    assert_eq!(NUM_BIT_MODEL_TOTAL_BITS, 11);
    assert_eq!(PROB_INIT_VAL, 1024u16);
    let _x: Prob = 0;
}

#[test]
fn test_prob_type_size() {
    assert_eq!(std::mem::size_of::<Prob>(), 2);
}

fn main() {}
