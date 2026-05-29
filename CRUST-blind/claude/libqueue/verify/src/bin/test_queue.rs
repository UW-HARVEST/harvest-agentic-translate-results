use libqueue::queue::Queue;

#[test]
fn test_new_queue_is_empty() {
    let q: Queue<i32> = Queue::new();
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_front_back_empty() {
    let q: Queue<i32> = Queue::new();
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_pop_empty() {
    let mut q: Queue<i32> = Queue::new();
    assert!(q.pop().is_none());
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_push_one() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    assert!(!q.is_empty());
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), Some(&10));
    assert_eq!(q.back(), Some(&10));
    // front and back should be equal when there's one element
    assert_eq!(q.front(), q.back());
}

#[test]
fn test_push_two() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(15);
    assert!(!q.is_empty());
    assert_eq!(q.size, 2);
    assert_eq!(q.front(), Some(&10));
    assert_eq!(q.back(), Some(&15));
}

#[test]
fn test_push_three_fifo_order() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.size, 3);
    assert_eq!(q.front(), Some(&1));
    assert_eq!(q.back(), Some(&3));

    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.size, 2);
    assert_eq!(q.front(), Some(&2));
    assert_eq!(q.back(), Some(&3));

    assert_eq!(q.pop(), Some(2));
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), Some(&3));
    assert_eq!(q.back(), Some(&3));

    assert_eq!(q.pop(), Some(3));
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_pop_then_empty_state() {
    let mut q: Queue<i32> = Queue::new();
    q.push(42);
    assert_eq!(q.pop(), Some(42));
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
    // After pop'ing the last element, front and back should be None
    assert!(q.front().is_none());
    assert!(q.back().is_none());
    // pop again on empty
    assert!(q.pop().is_none());
}

#[test]
fn test_push_pop_alternating() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    assert_eq!(q.pop(), Some(1));
    q.push(2);
    q.push(3);
    assert_eq!(q.pop(), Some(2));
    q.push(4);
    assert_eq!(q.front(), Some(&3));
    assert_eq!(q.back(), Some(&4));
    assert_eq!(q.size, 2);
    assert_eq!(q.pop(), Some(3));
    assert_eq!(q.pop(), Some(4));
    assert!(q.is_empty());
}

#[test]
fn test_push_after_complete_drain() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.pop();
    q.pop();
    assert!(q.is_empty());
    // Now push again - should work normally
    q.push(99);
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), Some(&99));
    assert_eq!(q.back(), Some(&99));
    q.push(100);
    assert_eq!(q.size, 2);
    assert_eq!(q.front(), Some(&99));
    assert_eq!(q.back(), Some(&100));
}

#[test]
fn test_free_clears_queue() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.size, 3);
    q.free();
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_free_empty_queue() {
    let mut q: Queue<i32> = Queue::new();
    q.free();
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
}

#[test]
fn test_push_after_free() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.free();
    q.push(99);
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), Some(&99));
    assert_eq!(q.back(), Some(&99));
    assert_eq!(q.pop(), Some(99));
    assert!(q.is_empty());
}

#[test]
fn test_string_values() {
    let mut q: Queue<String> = Queue::new();
    q.push("hello".to_string());
    q.push("world".to_string());
    assert_eq!(q.size, 2);
    assert_eq!(q.front(), Some(&"hello".to_string()));
    assert_eq!(q.back(), Some(&"world".to_string()));
    assert_eq!(q.pop(), Some("hello".to_string()));
    assert_eq!(q.pop(), Some("world".to_string()));
    assert!(q.is_empty());
}

#[test]
fn test_large_queue() {
    let mut q: Queue<u32> = Queue::new();
    for i in 0..1000u32 {
        q.push(i);
    }
    assert_eq!(q.size, 1000);
    assert_eq!(q.front(), Some(&0));
    assert_eq!(q.back(), Some(&999));

    for i in 0..1000u32 {
        assert_eq!(q.pop(), Some(i));
    }
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
}

#[test]
fn test_crud_replicates_c_test() {
    // Mirrors c_src/test/test.c crud_test()
    let mut q: Queue<i32> = Queue::new();

    // Queue Empty
    assert!(q.is_empty());

    let a: i32 = 10;
    let b: i32 = 15;

    // Queue Push (one element)
    q.push(a);
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), q.back());

    // Queue Push (second element)
    q.push(b);
    assert_eq!(q.size, 2);

    // Queue Front
    assert_eq!(q.front(), Some(&a));

    // Queue Back
    assert_eq!(q.back(), Some(&b));

    // Queue Pop
    assert_eq!(q.pop(), Some(a));
    assert_eq!(q.size, 1);

    // Queue Free
    q.free();
    assert!(q.is_empty());
}

#[test]
fn test_size_consistency() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.size, 0);
    for i in 1..=5 {
        q.push(i);
        assert_eq!(q.size, i as usize);
    }
    for i in (0..5).rev() {
        q.pop();
        assert_eq!(q.size, i as usize);
    }
}

#[test]
fn test_back_unchanged_when_only_front_popped() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.back(), Some(&3));
    q.pop(); // pops 1
    assert_eq!(q.back(), Some(&3));
    q.pop(); // pops 2
    assert_eq!(q.back(), Some(&3));
    assert_eq!(q.front(), Some(&3));
}

fn main() {}
