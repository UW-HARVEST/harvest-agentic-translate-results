use Graph_recogniser::countof;

#[test]
fn test_countof_array() {
    let arr = [1, 2, 3, 4, 5];
    assert_eq!(countof!(arr), 5);
}

#[test]
fn test_countof_empty_array() {
    let arr: [i32; 0] = [];
    assert_eq!(countof!(arr), 0);
}

#[test]
fn test_countof_string_array() {
    let arr = ["a", "b", "c"];
    assert_eq!(countof!(arr), 3);
}

#[test]
fn test_countof_vec() {
    let v = vec![10, 20, 30, 40];
    assert_eq!(countof!(v), 4);
}

fn main() {}
