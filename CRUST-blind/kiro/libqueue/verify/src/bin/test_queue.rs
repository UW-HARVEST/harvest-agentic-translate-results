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
}

#[test]
fn test_push_two() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(20);
    assert_eq!(q.size, 2);
    assert_eq!(*q.front().unwrap(), 10);
    assert_eq!(*q.back().unwrap(), 20);
}

#[test]
fn test_push_three() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(20);
    q.push(30);
    assert_eq!(q.size, 3);
    assert_eq!(*q.front().unwrap(), 10);
    assert_eq!(*q.back().unwrap(), 30);
}

#[test]
fn test_pop_fifo_order() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.push(20);
    q.push(30);

    let v1 = q.pop();
    assert_eq!(v1, Some(10));
    assert_eq!(q.size, 2);
    assert_eq!(*q.front().unwrap(), 20);
    assert_eq!(*q.back().unwrap(), 30);

    let v2 = q.pop();
    assert_eq!(v2, Some(20));
    assert_eq!(q.size, 1);
    assert_eq!(*q.front().unwrap(), 30);
    assert_eq!(*q.back().unwrap(), 30);

    let v3 = q.pop();
    assert_eq!(v3, Some(30));
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_pop_empty() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.pop(), None);
    assert_eq!(q.size, 0);
}

#[test]
fn test_pop_then_pop_empty() {
    let mut q: Queue<i32> = Queue::new();
    q.push(5);
    assert_eq!(q.pop(), Some(5));
    assert_eq!(q.pop(), None);
    assert_eq!(q.size, 0);
}

#[test]
fn test_push_after_emptying() {
    let mut q: Queue<i32> = Queue::new();
    q.push(10);
    q.pop();
    assert!(q.is_empty());
    assert!(q.front().is_none());
    assert!(q.back().is_none());

    q.push(42);
    assert_eq!(q.size, 1);
    assert_eq!(*q.front().unwrap(), 42);
    assert_eq!(*q.back().unwrap(), 42);
}

#[test]
fn test_free() {
    let mut q: Queue<i32> = Queue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.size, 3);
    q.free();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
    assert!(q.front().is_none());
}

#[test]
fn test_free_single_element() {
    let mut q: Queue<i32> = Queue::new();
    q.push(99);
    q.free();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_free_empty_queue() {
    let mut q: Queue<i32> = Queue::new();
    q.free();
    assert_eq!(q.size, 0);
    assert!(q.is_empty());
}

#[test]
fn test_is_empty_transitions() {
    let mut q: Queue<i32> = Queue::new();
    assert!(q.is_empty());
    q.push(1);
    assert!(!q.is_empty());
    q.pop();
    assert!(q.is_empty());
}

#[test]
fn test_many_elements_fifo() {
    let mut q: Queue<i32> = Queue::new();
    for i in 0..10 {
        q.push(i);
    }
    assert_eq!(q.size, 10);
    assert_eq!(*q.front().unwrap(), 0);
    assert_eq!(*q.back().unwrap(), 9);
    for i in 0..10 {
        assert_eq!(q.pop(), Some(i));
    }
    assert!(q.is_empty());
    assert_eq!(q.pop(), None);
}

fn main() {}
