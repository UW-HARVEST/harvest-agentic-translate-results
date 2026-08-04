use Megalania::perplexity_table::LOG2_LOOKUP;

#[test]
fn test_lookup_size() {
    assert_eq!(LOG2_LOOKUP.len(), 2048);
}

#[test]
fn test_known_values() {
    assert_eq!(LOG2_LOOKUP[0], 0);
    assert_eq!(LOG2_LOOKUP[1], 22528);
    assert_eq!(LOG2_LOOKUP[2], 20480);
    assert_eq!(LOG2_LOOKUP[1024], 2048);
    assert_eq!(LOG2_LOOKUP[2047], 1);
}

fn main() {}
