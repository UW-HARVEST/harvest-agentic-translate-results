use Graph_recogniser::countof;

#[test]
fn test_countof_array() {
    let arr = [1, 2, 3, 4, 5];
    assert_eq!(countof!(arr), 5);
}

#[test]
fn test_countof_empty() {
    let arr: [i32; 0] = [];
    assert_eq!(countof!(arr), 0);
}

#[test]
fn test_countof_single() {
    let arr = [42];
    assert_eq!(countof!(arr), 1);
}

fn main() {}
