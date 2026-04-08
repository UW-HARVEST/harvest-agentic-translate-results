use Graph_recogniser::countof;

#[test]
fn test_countof() {
    let arr = [1, 2, 3, 4, 5];
    assert_eq!(countof!(arr), 5);
    let empty: [i32; 0] = [];
    assert_eq!(countof!(empty), 0);
    let single = [42];
    assert_eq!(countof!(single), 1);
}

fn main() {}
