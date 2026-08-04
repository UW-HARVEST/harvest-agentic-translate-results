use Megalania::probability::{NUM_BIT_MODEL_TOTAL_BITS, PROB_INIT_VAL};

#[test]
fn test_num_bit_model_total_bits() {
    assert_eq!(NUM_BIT_MODEL_TOTAL_BITS, 11);
}

#[test]
fn test_prob_init_val() {
    assert_eq!(PROB_INIT_VAL, 1024);
    assert_eq!(PROB_INIT_VAL, (1u16 << NUM_BIT_MODEL_TOTAL_BITS) / 2);
}

#[test]
fn test_prob_size() {
    // Prob is u16 (alias)
    let p: Megalania::probability::Prob = 1024;
    assert_eq!(p, 1024);
    assert_eq!(std::mem::size_of::<Megalania::probability::Prob>(), 2);
}

fn main() {}
