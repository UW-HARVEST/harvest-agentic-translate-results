use fslib::queue::Queue;

#[test]
fn test_basic() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.len(), 0);
    q.enqueue(10);
    q.enqueue(20);
    q.enqueue(30);
    assert_eq!(q.len(), 3);
    assert_eq!(q.dequeue(), Some(10));
    assert_eq!(q.dequeue(), Some(20));
    assert_eq!(q.len(), 1);
    assert_eq!(q.dequeue(), Some(30));
    assert_eq!(q.dequeue(), None);
}

#[test]
fn test_empty() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    q.empty();
    assert_eq!(q.len(), 0);
    assert_eq!(q.dequeue(), None);
}

fn main() {}
