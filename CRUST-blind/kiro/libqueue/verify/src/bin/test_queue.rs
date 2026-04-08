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
fn test_push_single() {
    let mut q = Queue::new();
    q.push(10);
    assert_eq!(q.size, 1);
    assert!(!q.is_empty());
    // front == back when single element
    assert_eq!(q.front(), q.back());
    assert_eq!(q.front(), Some(&10));
}

#[test]
fn test_push_two_elements() {
    let mut q = Queue::new();
    q.push(10);
    q.push(15);
    assert_eq!(q.size, 2);
    assert_eq!(q.front(), Some(&10));
    assert_eq!(q.back(), Some(&15));
}

#[test]
fn test_pop_returns_front() {
    let mut q = Queue::new();
    q.push(10);
    q.push(15);
    assert_eq!(q.pop(), Some(10));
    assert_eq!(q.size, 1);
}

#[test]
fn test_pop_empty_returns_none() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.pop(), None);
}

#[test]
fn test_front_empty_returns_none() {
    let q: Queue<i32> = Queue::new();
    assert_eq!(q.front(), None);
}

#[test]
fn test_back_empty_returns_none() {
    let q: Queue<i32> = Queue::new();
    assert_eq!(q.back(), None);
}

#[test]
fn test_fifo_order() {
    let mut q = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.pop(), Some(2));
    assert_eq!(q.pop(), Some(3));
    assert_eq!(q.pop(), None);
    assert!(q.is_empty());
}

#[test]
fn test_pop_until_empty_then_push() {
    let mut q = Queue::new();
    q.push(5);
    q.push(10);
    q.pop();
    q.pop();
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
    // Push again after draining
    q.push(20);
    assert_eq!(q.size, 1);
    assert_eq!(q.front(), Some(&20));
    assert_eq!(q.back(), Some(&20));
}

#[test]
fn test_free_clears_queue() {
    let mut q = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    q.free();
    assert!(q.is_empty());
    assert_eq!(q.size, 0);
}

#[test]
fn test_pop_single_element_clears_tail() {
    let mut q = Queue::new();
    q.push(42);
    assert_eq!(q.pop(), Some(42));
    assert!(q.back().is_none());
    assert!(q.front().is_none());
    assert_eq!(q.size, 0);
}

#[test]
fn test_size_tracks_correctly() {
    let mut q = Queue::new();
    for i in 0..5 {
        q.push(i);
        assert_eq!(q.size, i + 1);
    }
    for i in (1..=5).rev() {
        q.pop();
        assert_eq!(q.size, i - 1);
    }
}

#[test]
fn test_front_back_after_pop() {
    let mut q = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    q.pop(); // remove 1
    assert_eq!(q.front(), Some(&2));
    assert_eq!(q.back(), Some(&3));
}

#[test]
fn test_string_values() {
    let mut q = Queue::new();
    q.push("hello");
    q.push("world");
    assert_eq!(q.front(), Some(&"hello"));
    assert_eq!(q.back(), Some(&"world"));
    assert_eq!(q.pop(), Some("hello"));
    assert_eq!(q.pop(), Some("world"));
}

fn main() {}
