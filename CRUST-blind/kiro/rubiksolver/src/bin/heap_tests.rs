use rubiksolver::heap::Heap;

fn main() {
    let mut heap: Heap<String> = Heap::new(10, |a: &String, b: &String| a < b);

    println!("Testing empty heap...");
    assert!(heap.is_empty());

    println!("Adding an item...");
    heap.insert("charlie".to_string());

    println!("Adding an item...");
    heap.insert("alpha".to_string());

    println!("Adding an item...");
    heap.insert("bravo".to_string());

    println!("Testing non-empty heap...");
    assert!(!heap.is_empty());

    println!("Testing output...");
    assert_eq!(heap.delete_min().unwrap(), "alpha");
    assert_eq!(heap.delete_min().unwrap(), "bravo");
    assert_eq!(heap.delete_min().unwrap(), "charlie");

    println!("Testing empty heap...");
    assert!(heap.is_empty());

    // Integer test
    let mut heap2: Heap<i32> = Heap::new(10, |a: &i32, b: &i32| a < b);
    for i in (11..=30).rev() {
        heap2.insert(i);
    }
    assert_eq!(heap2.delete_min().unwrap(), 11);
    println!("Passed all tests");
}
