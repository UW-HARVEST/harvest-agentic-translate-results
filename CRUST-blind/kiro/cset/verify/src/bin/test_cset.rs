use cset::cset::Cset;

#[test]
fn test_init() {
    let s: Cset<i32> = Cset::new();
    assert_eq!(s.get_size(), 0);
    assert_eq!(s.capacity(), 2);
    assert_eq!(s.get_seed(), 2718182);
    assert_eq!(s.get_max_load_factor(), 0.7);
    assert_eq!(s.get_min_load_factor(), 0.2);
}

#[test]
fn test_add() {
    let mut s: Cset<i32> = Cset::new();
    s.add(34);
    assert_eq!(s.get_size(), 1);
    s.add(35);
    assert_eq!(s.get_size(), 2);
    s.add(36);
    s.add(37);
    s.add(38);
    assert_eq!(s.get_size(), 5);
    assert_eq!(s.capacity(), 8);
}

#[test]
fn test_contains() {
    let mut s: Cset<i32> = Cset::new();
    s.add(34);
    s.add(36);
    s.remove(36);
    assert_eq!(s.contains(&12), false);
    assert_eq!(s.contains(&34), true);
    s.add(50);
    assert_eq!(s.contains(&45), false);
    assert_eq!(s.get_size(), 2);
}

#[test]
fn test_unique() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(57);
    assert_eq!(s.get_size(), 3);
    s.add(45);
    assert_eq!(s.get_size(), 3);
}

#[derive(Copy, Clone, Default)]
struct Node {
    x: i32,
    y: i32,
}

#[test]
fn test_struct() {
    let mut s: Cset<Node> = Cset::new();
    s.add(Node { x: 4, y: 4 });
    assert_eq!(s.get_size(), 1);
    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.get_size(), 2);
    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.get_size(), 2);
    s.add(Node { x: 5, y: 8 });
    assert_eq!(s.get_size(), 3);
}

#[test]
fn test_remove() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(34);
    s.add(10);
    assert_eq!(s.get_size(), 3);
    s.remove(45);
    assert_eq!(s.get_size(), 2);
    s.remove(45);
    assert_eq!(s.get_size(), 2);
    s.remove(34);
    assert_eq!(s.get_size(), 1);

    let items = s.iter();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], 10);

    s.remove(10);
    assert_eq!(s.get_size(), 0);
}

#[test]
fn test_resize() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..1500 {
        s.add(i);
    }
    assert_eq!(s.get_size(), 1500);
    assert_eq!(s.capacity(), 4096);
}

#[test]
fn test_default_bytes_comparator() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(67);

    assert_eq!(s.contains(&45), true);
    assert_eq!(s.contains(&68), false);
    assert_eq!(s.contains(&46), true);

    s.remove(46);
    assert_eq!(s.contains(&46), false);
    s.remove(46);
    assert_eq!(s.contains(&46), false);
    assert_eq!(s.get_size(), 2);

    s.remove(45);
    assert_eq!(s.get_size(), 1);
    s.remove(67);
    assert_eq!(s.get_size(), 0);
    s.remove(67);
    assert_eq!(s.get_size(), 0);

    for i in 0..2000 {
        s.add(i);
    }
    assert_eq!(s.get_size(), 2000);
}

#[test]
fn test_custom_comparator() {
    let mut s: Cset<Node> = Cset::new();
    s.set_comparator(|a: &Node, b: &Node| a.x == b.x);
    s.add(Node { x: 4, y: 4 });
    s.add(Node { x: 4, y: 4 });
    assert_eq!(s.get_size(), 1);
    s.add(Node { x: 1, y: 2 });
    assert_eq!(s.get_size(), 2);
    s.remove(Node { x: 1, y: 45 });
    assert_eq!(s.get_size(), 1);
}

#[test]
fn test_clear() {
    let mut s: Cset<i32> = Cset::new();
    s.add(12);
    s.add(14);
    s.add(15);
    assert_eq!(s.get_size(), 3);
    s.clear();
    assert_eq!(s.get_size(), 0);
    assert_eq!(s.capacity(), 2);
    s.add(45);
    assert_eq!(s.get_size(), 1);
}

#[test]
fn test_intersection() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut r: Cset<i32> = Cset::new();

    a.add(12); a.add(13); a.add(14);
    b.add(12); b.add(13); b.add(16);
    r.intersect(&a, &b);
    assert_eq!(r.get_size(), 2);

    b.add(14);
    r.intersect(&a, &b);
    assert_eq!(r.get_size(), 3);
}

#[test]
fn test_union() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut r: Cset<i32> = Cset::new();

    a.add(34); a.add(25); a.add(12);
    b.add(1); b.add(4); b.add(34);
    r.union(&a, &b);
    assert_eq!(r.get_size(), 5);

    b.add(100);
    r.union(&a, &b);
    assert_eq!(r.get_size(), 6);
}

#[test]
fn test_disjoint() {
    let mut a: Cset<i8> = Cset::new();
    let mut b: Cset<i8> = Cset::new();

    a.add(b'a' as i8); a.add(b'b' as i8);
    b.add(b'c' as i8); b.add(b'd' as i8);
    assert_eq!(a.is_disjoint(&b), true);

    b.add(b'a' as i8);
    assert_eq!(a.is_disjoint(&b), false);
}

#[test]
fn test_difference() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut r: Cset<i32> = Cset::new();

    r.difference(&a, &b);
    assert_eq!(r.get_size(), 0);

    a.add(45); a.add(46); a.add(58);
    b.add(12); b.add(11); b.add(45);
    r.difference(&a, &b);
    assert_eq!(r.get_size(), 2);
    assert_eq!(r.contains(&46), true);
    assert_eq!(r.contains(&58), true);
    assert_eq!(r.contains(&45), false);

    r.clear();
    b.add(46); b.add(58);
    r.difference(&a, &b);
    assert_eq!(r.get_size(), 0);

    r.difference(&b, &a);
    assert_eq!(r.get_size(), 2);
}

#[test]
fn test_iteration() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..3200 {
        s.add(i);
    }
    assert_eq!(s.get_size(), 3200);
    let items = s.iter();
    assert_eq!(items.len(), 3200);
    for val in &items {
        assert_eq!(s.contains(val), true);
    }
}

fn main() {}
