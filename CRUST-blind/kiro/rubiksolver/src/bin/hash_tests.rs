use rubiksolver::hash::Hash;

fn main() {
    let mut hash: Hash<String> = Hash::new(255, |e: &String| e.as_bytes()[0] as u32);
    let eq = |a: &String, b: &String| a == b;

    assert!(!hash.element_exists(&"Hello".to_string(), eq));
    println!("Inserting element 'Hello'...");
    hash.insert("Hello".to_string(), eq);
    assert!(hash.element_exists(&"Hello".to_string(), eq));
    println!("Inserting element 'Hi'...");
    hash.insert("Hi".to_string(), eq);
    assert!(hash.element_exists(&"Hi".to_string(), eq));
    println!("Removing element 'Hello'...");
    hash.delete(&"Hello".to_string(), eq);
    assert!(!hash.element_exists(&"Hello".to_string(), eq));
    println!("All tests passed successfully.");
}
