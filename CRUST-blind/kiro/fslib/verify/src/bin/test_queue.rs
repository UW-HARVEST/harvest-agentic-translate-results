use fslib::queue::Queue;

#[test]
fn test_fifo_order() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(10);
    q.enqueue(20);
    q.enqueue(30);
    assert_eq!(q.len(), 3);
    assert_eq!(q.dequeue(), Some(10));
    assert_eq!(q.dequeue(), Some(20));
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

#[test]
fn test_empty_queue_dequeue() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.dequeue(), None);
}

#[test]
fn test_single_element() {
    let mut q: Queue<u32> = Queue::new();
    q.enqueue(42);
    assert_eq!(q.len(), 1);
    assert_eq!(q.dequeue(), Some(42));
    assert_eq!(q.len(), 0);
}

fn main() {}
