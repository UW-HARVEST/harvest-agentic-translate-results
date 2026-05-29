use bhshell::dynamicarr::{destroy_args, get_args, get_string, ArgList, Str, DA_BUFFER_SIZE};

#[test]
fn test_da_buffer_size_constant() {
    assert_eq!(DA_BUFFER_SIZE, 16);
}

#[test]
fn test_str_default_init() {
    let s = Str::default();
    assert_eq!(s.position, 0);
    assert_eq!(s.bufsize, 0);
    assert_eq!(s.items.len(), 0);
    assert_eq!(s.items, "");
}

#[test]
fn test_str_append_grows_buffer() {
    let mut s = Str::default();
    s.append('a');
    assert_eq!(s.position, 1);
    assert_eq!(s.bufsize, 16);
    assert_eq!(s.items, "a");

    s.append('b');
    assert_eq!(s.position, 2);
    assert_eq!(s.bufsize, 16);
    assert_eq!(s.items, "ab");

    s.append('c');
    assert_eq!(s.position, 3);
    assert_eq!(s.items, "abc");
    assert_eq!(s.bufsize, 16);
}

#[test]
fn test_str_append_doubles_buffer_after_capacity() {
    // Append 17 chars: first 16 fit in initial bufsize=16; 17th doubles to 32.
    let mut s = Str::default();
    for _ in 0..16 {
        s.append('x');
    }
    assert_eq!(s.position, 16);
    assert_eq!(s.bufsize, 16);

    s.append('y');
    assert_eq!(s.position, 17);
    assert_eq!(s.bufsize, 32);
    assert_eq!(s.items.len(), 17);
}

#[test]
fn test_get_string_basic() {
    let mut s = Str::default();
    s.append('h');
    s.append('i');
    s.append('!');
    let out = get_string(&s);
    assert_eq!(out, "hi!");
    assert_eq!(out.len(), 3);
}

#[test]
fn test_get_string_empty() {
    let s = Str::default();
    let out = get_string(&s);
    assert_eq!(out, "");
    assert_eq!(out.len(), 0);
}

#[test]
fn test_get_string_truncates_to_position() {
    // Make a Str whose items has more chars than position; get_string should
    // only copy `position` characters from items.
    let mut s = Str::default();
    s.items = "abcdef".to_string();
    s.position = 3;
    s.bufsize = 16;
    let out = get_string(&s);
    assert_eq!(out, "abc");
    assert_eq!(out.len(), 3);
}

#[test]
fn test_arglist_default_init() {
    let l = ArgList::default();
    assert_eq!(l.position, 0);
    assert_eq!(l.bufsize, 0);
    assert!(l.items.is_empty());
}

#[test]
fn test_arglist_append_grows_buffer() {
    let mut l = ArgList::default();
    l.append("alpha".to_string());
    assert_eq!(l.position, 1);
    assert_eq!(l.bufsize, 16);
    assert_eq!(l.items.len(), 1);
    assert_eq!(l.items[0], "alpha");

    l.append("beta".to_string());
    assert_eq!(l.position, 2);
    assert_eq!(l.bufsize, 16);
    assert_eq!(l.items.len(), 2);
    assert_eq!(l.items[1], "beta");
}

#[test]
fn test_arglist_append_doubles_buffer() {
    let mut l = ArgList::default();
    for i in 0..16 {
        l.append(format!("s{}", i));
    }
    assert_eq!(l.position, 16);
    assert_eq!(l.bufsize, 16);

    l.append("overflow".to_string());
    assert_eq!(l.position, 17);
    assert_eq!(l.bufsize, 32);
    assert_eq!(l.items[16], "overflow");
}

#[test]
fn test_get_args_empty() {
    let l = ArgList::default();
    let args = get_args(&l);
    assert!(args.is_empty());
    assert_eq!(args.len(), 0);
}

#[test]
fn test_get_args_basic() {
    let mut l = ArgList::default();
    l.append("hello".to_string());
    l.append("world".to_string());
    let args = get_args(&l);
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "hello");
    assert_eq!(args[1], "world");
}

#[test]
fn test_get_args_three_items() {
    let mut l = ArgList::default();
    l.append("a".to_string());
    l.append("b".to_string());
    l.append("c".to_string());
    let args = get_args(&l);
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "a");
    assert_eq!(args[1], "b");
    assert_eq!(args[2], "c");
}

#[test]
fn test_destroy_args_does_not_panic() {
    let v = vec!["a".to_string(), "b".to_string()];
    destroy_args(v);
    let empty: Vec<String> = Vec::new();
    destroy_args(empty);
}

fn main() {}
