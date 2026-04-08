use blt::cbt::*;

#[test]
fn test_cbt_new_empty() {
    let cbt = Cbt::cbt_new();
    assert_eq!(cbt.cbt_size(), 0);
    assert!(cbt.cbt_first().is_none());
    assert!(cbt.cbt_last().is_none());
    assert!(!cbt.cbt_has("x"));
}

#[test]
fn test_cbt_put_at_and_has() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(42i32), "hello");
    assert_eq!(cbt.cbt_size(), 1);
    assert!(cbt.cbt_has("hello"));
    assert!(!cbt.cbt_has("world"));
}

#[test]
fn test_cbt_sorted_order() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");
    cbt.cbt_put_at(Box::new(()), "xyz");
    assert_eq!(cbt.cbt_size(), 4);

    // Iterate via first/next
    let mut keys = Vec::new();
    let mut it = cbt.cbt_first();
    while let Some(ref cur) = it {
        keys.push(cur.key.clone());
        it = Cbt::cbt_next(cur);
    }
    assert_eq!(keys, vec!["abc", "hello", "world", "xyz"]);
}

#[test]
fn test_cbt_first_last() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");
    cbt.cbt_put_at(Box::new(()), "xyz");
    assert_eq!(cbt.cbt_first().unwrap().key, "abc");
    assert_eq!(cbt.cbt_last().unwrap().key, "xyz");
}

#[test]
fn test_cbt_at() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    assert!(cbt.cbt_at("hello").is_some());
    assert_eq!(cbt.cbt_at("hello").unwrap().key, "hello");
    assert!(cbt.cbt_at("missing").is_none());
}

#[test]
fn test_cbt_get_at() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    assert!(cbt.cbt_get_at("hello").is_some());
    assert!(cbt.cbt_get_at("missing").is_none());
}

#[test]
fn test_cbt_insert() {
    let mut cbt = Cbt::cbt_new();
    let (is_new, leaf) = cbt.cbt_insert("hello");
    assert!(is_new);
    assert_eq!(leaf.key, "hello");

    let (is_new2, leaf2) = cbt.cbt_insert("hello");
    assert!(!is_new2);
    assert_eq!(leaf2.key, "hello");

    let (is_new3, leaf3) = cbt.cbt_insert("world");
    assert!(is_new3);
    assert_eq!(leaf3.key, "world");
    assert_eq!(cbt.cbt_size(), 2);
}

#[test]
fn test_cbt_remove() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");
    cbt.cbt_put_at(Box::new(()), "xyz");

    let removed = cbt.cbt_remove("abc");
    assert!(removed.is_some());
    assert_eq!(cbt.cbt_size(), 3);
    assert!(!cbt.cbt_has("abc"));

    // Verify remaining order
    let mut keys = Vec::new();
    let mut it = cbt.cbt_first();
    while let Some(ref cur) = it {
        keys.push(cur.key.clone());
        it = Cbt::cbt_next(cur);
    }
    assert_eq!(keys, vec!["hello", "world", "xyz"]);
}

#[test]
fn test_cbt_remove_single() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "only");
    let removed = cbt.cbt_remove("only");
    assert!(removed.is_some());
    assert_eq!(cbt.cbt_size(), 0);
    assert!(cbt.cbt_first().is_none());
}

#[test]
fn test_cbt_remove_all() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");
    cbt.cbt_remove_all();
    assert_eq!(cbt.cbt_size(), 0);
    assert!(cbt.cbt_first().is_none());
    assert!(cbt.cbt_last().is_none());
}

#[test]
fn test_cbt_remove_all_empty() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_remove_all();
    assert_eq!(cbt.cbt_size(), 0);
}

#[test]
fn test_cbt_forall() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");

    let mut keys = Vec::new();
    cbt.cbt_forall(|leaf| keys.push(leaf.key.clone()));
    assert_eq!(keys, vec!["abc", "hello", "world"]);
}

#[test]
fn test_cbt_forall_at() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");

    let mut keys = Vec::new();
    cbt.cbt_forall_at(|_data, key| keys.push(key.to_string()));
    assert_eq!(keys, vec!["hello", "world"]);
}

#[test]
fn test_cbt_key() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    let leaf = cbt.cbt_first().unwrap();
    assert_eq!(cbt.cbt_key(&leaf), "hello");
}

#[test]
fn test_cbt_overhead_empty() {
    let cbt = Cbt::cbt_new();
    assert_eq!(cbt.cbt_overhead(), std::mem::size_of::<Cbt>());
}

#[test]
fn test_cbt_overhead_items() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    cbt.cbt_put_at(Box::new(()), "abc");
    cbt.cbt_put_at(Box::new(()), "xyz");
    cbt.cbt_remove("abc");
    // 3 items: sizeof(Cbt) + 3*sizeof(CbtLeaf) + 2*sizeof(CbtNode)
    let expected = std::mem::size_of::<Cbt>()
        + 3 * std::mem::size_of::<CbtLeaf>()
        + 2 * std::mem::size_of::<CbtNode>();
    assert_eq!(cbt.cbt_overhead(), expected);
}

#[test]
fn test_cbt_put_with() {
    let mut cbt = Cbt::cbt_new();
    let leaf = cbt.cbt_put_with(|_| Box::new(10i32), "hello");
    assert_eq!(leaf.key, "hello");
    assert_eq!(cbt.cbt_size(), 1);

    // put_with on existing key calls fn with old data
    let leaf2 = cbt.cbt_put_with(|_old| Box::new(20i32), "hello");
    assert_eq!(leaf2.key, "hello");
    assert_eq!(cbt.cbt_size(), 1);
}

#[test]
fn test_cbt_put_at_overwrite() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "hello");
    cbt.cbt_put_at(Box::new(2i32), "hello");
    assert_eq!(cbt.cbt_size(), 1);
}

#[test]
fn test_cbt_remove_all_with() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(()), "hello");
    cbt.cbt_put_at(Box::new(()), "world");
    let mut visited = Vec::new();
    cbt.cbt_remove_all_with(|_data, key| visited.push(key.to_string()));
    assert_eq!(visited, vec!["hello", "world"]);
    assert_eq!(cbt.cbt_size(), 0);
}

#[test]
fn test_cbt_new_u() {
    let mut cbt = Cbt::cbt_new_u(4);
    assert_eq!(cbt.cbt_size(), 0);
    cbt.cbt_delete();
}

#[test]
fn test_cbt_new_enc() {
    let mut cbt = Cbt::cbt_new_enc();
    assert_eq!(cbt.cbt_size(), 0);
    cbt.cbt_delete();
}

#[test]
fn test_cbt_delete() {
    let cbt = Cbt::cbt_new();
    cbt.cbt_delete(); // should not panic
}

fn main() {}
