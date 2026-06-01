use libqueue::queue::Queue;

#[test]
fn test_new_queue() {
    let q: Queue<i32> = Queue::new();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_push_one() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    assert_eq!(q.size, 1);
    assert!(!q.is_empty());
    assert_eq!(*q.front().unwrap(), 10);
    assert_eq!(*q.back().unwrap(), 10);
    // front and back point to the same node
    let front_ptr: *const i32 = q.front().unwrap();
    let back_ptr: *const i32 = q.back().unwrap();
    assert_eq!(front_ptr, back_ptr);
}

#[test]
fn test_push_two() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(15);
    assert_eq!(q.size, 2);
    assert_eq!(*q.front().unwrap(), 10);
    assert_eq!(*q.back().unwrap(), 15);
}

#[test]
fn test_pop_one_of_two() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(15);
    let popped = q.pop().unwrap();
    assert_eq!(popped, 10);
    assert_eq!(q.size, 1);
    assert_eq!(*q.front().unwrap(), 15);
    assert_eq!(*q.back().unwrap(), 15);
}

#[test]
fn test_pop_to_empty() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(15);
    assert_eq!(q.pop().unwrap(), 10);
    assert_eq!(q.pop().unwrap(), 15);
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_pop_empty_returns_none() {
    let mut q: Queue<i32> = Queue::new();
    assert!(q.pop().is_none());
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_many_push_then_pop() {
    let mut q: Queue<i32> = Queue::new();
    let values = [1, 2, 3, 4, 5];
    for &v in &values {
        q.push(v);
    }
    assert_eq!(q.size, 5);
    assert_eq!(*q.front().unwrap(), 1);
    assert_eq!(*q.back().unwrap(), 5);

    let expected_pops = [1, 2, 3, 4, 5];
    for &expected in &expected_pops {
        assert_eq!(q.pop().unwrap(), expected);
    }
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_push_pop_alternating() {
    let mut q: Queue<i32> = Queue::new();
    q.push(100);
    let popped = q.pop().unwrap();
    assert_eq!(popped, 100);
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    q.push(200);
    assert_eq!(q.size, 1);
    assert_eq!(*q.front().unwrap(), 200);
    assert_eq!(*q.back().unwrap(), 200);
}

#[test]
fn test_is_empty_initial() {
    let q: Queue<i32> = Queue::new();
    assert!(q.is_empty());
}

#[test]
fn test_is_empty_after_push() {
    let mut q: Queue<i32> = Queue::new();
    q.push(42);
    assert!(!q.is_empty());
}

#[test]
fn test_front_empty() {
    let q: Queue<i32> = Queue::new();
    assert!(q.front().is_none());
}

#[test]
fn test_back_empty() {
    let q: Queue<i32> = Queue::new();
    assert!(q.back().is_none());
}

#[test]
fn test_free_resets_queue() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.size, 3);
    q.free();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    assert!(q.front().is_none());
    assert!(q.back().is_none());
}

#[test]
fn test_free_empty_queue() {
    let mut q: Queue<i32> = Queue::new();
    q.free();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_push_after_free() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.free();
    q.push(99);
    assert_eq!(q.size, 1);
    assert_eq!(*q.front().unwrap(), 99);
    assert_eq!(*q.back().unwrap(), 99);
}

#[test]
fn test_with_strings() {
    let mut q: Queue<String> = Queue::new();
    q.push(String::from("hello"));
    q.push(String::from("world"));
    assert_eq!(q.size, 2);
    assert_eq!(q.front().unwrap(), "hello");
    assert_eq!(q.back().unwrap(), "world");
    assert_eq!(q.pop().unwrap(), "hello");
    assert_eq!(q.pop().unwrap(), "world");
    assert!(q.is_empty());
}

#[test]
fn test_size_progression() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.size, 0);
    q.push(1);
    assert_eq!(q.size, 1);
    q.push(2);
    assert_eq!(q.size, 2);
    q.push(3);
    assert_eq!(q.size, 3);
    q.pop();
    assert_eq!(q.size, 2);
    q.pop();
    assert_eq!(q.size, 1);
    q.pop();
    assert_eq!(q.size, 0);
    q.pop(); // pop from empty - should not underflow
    assert_eq!(q.size, 0);
}

fn main() {}
