use Megalania::perplexity_table::LOG2_LOOKUP;

#[test]
fn test_table_spot_checks() {
    // C ground truth
    assert_eq!(LOG2_LOOKUP[0], 0);
    assert_eq!(LOG2_LOOKUP[1], 22528);
    assert_eq!(LOG2_LOOKUP[1024], 2048);
    assert_eq!(LOG2_LOOKUP[2047], 1);
    assert_eq!(LOG2_LOOKUP[100], 8921);
    assert_eq!(LOG2_LOOKUP[500], 4166);
    assert_eq!(LOG2_LOOKUP[1500], 920);
}

#[test]
fn test_table_length() {
    assert_eq!(LOG2_LOOKUP.len(), 2048);
}

fn main() {}
