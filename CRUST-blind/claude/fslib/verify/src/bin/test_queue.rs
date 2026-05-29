use fslib::queue::Queue;

#[test]
fn test_queue_new_is_empty() {
    let q: Queue<i32> = Queue::new();
    assert_eq!(q.len(), 0);
    assert_eq!(q.items.len(), 0);
}

#[test]
fn test_queue_enqueue_dequeue_fifo() {
    let mut q: Queue<i32> = Queue::new();
    for i in 0..5 {
        q.enqueue(i);
    }
    assert_eq!(q.len(), 5);
    let mut out = Vec::new();
    while let Some(v) = q.dequeue() {
        out.push(v);
    }
    assert_eq!(out, vec![0, 1, 2, 3, 4]);
    assert_eq!(q.len(), 0);
}

#[test]
fn test_queue_dequeue_empty() {
    let mut q: Queue<i32> = Queue::new();
    assert!(q.dequeue().is_none());
}

#[test]
fn test_queue_empty_method() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    q.enqueue(3);
    q.empty();
    assert_eq!(q.len(), 0);
    assert!(q.dequeue().is_none());
}

#[test]
fn test_queue_with_strings() {
    let mut q: Queue<String> = Queue::new();
    q.enqueue("hello".to_string());
    q.enqueue("world".to_string());
    assert_eq!(q.dequeue(), Some("hello".to_string()));
    assert_eq!(q.dequeue(), Some("world".to_string()));
    assert!(q.dequeue().is_none());
}

#[test]
fn test_queue_interleaved() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    assert_eq!(q.dequeue(), Some(1));
    q.enqueue(3);
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), Some(3));
    assert!(q.dequeue().is_none());
}

fn main() {}
