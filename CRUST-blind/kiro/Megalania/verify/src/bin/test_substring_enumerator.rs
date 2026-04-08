use Megalania::substring_enumerator::SubstringEnumerator;

fn run_with_stack<F: FnOnce() + Send + 'static>(f: F) {
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder.spawn(f).unwrap();
    handler.join().unwrap();
}

#[test]
fn test_hello_hello() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let em = SubstringEnumerator::new(data, 2, 273);

        let expected_counts = [0, 0, 0, 0, 0, 0, 4, 3, 2, 1, 0];
        let expected_results: Vec<Vec<(usize, usize)>> = vec![
            vec![], vec![], vec![], vec![], vec![], vec![],
            vec![(0, 2), (0, 3), (0, 4), (0, 5)],
            vec![(1, 2), (1, 3), (1, 4)],
            vec![(2, 2), (2, 3)],
            vec![(3, 2)],
            vec![],
        ];

        for pos in 0..11 {
            let mut results: Vec<(usize, usize)> = Vec::new();
            em.for_each(pos, |offset, length| {
                results.push((offset, length));
            });
            assert_eq!(results.len(), expected_counts[pos],
                "pos={} expected {} matches, got {}", pos, expected_counts[pos], results.len());
            assert_eq!(results, expected_results[pos], "pos={} results mismatch", pos);
        }
    });
}

#[test]
fn test_hello_hello_max3() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let em = SubstringEnumerator::new(data, 2, 3);

        let expected_counts = [0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 0];
        for pos in 0..11 {
            let mut count = 0;
            em.for_each(pos, |_offset, _length| {
                count += 1;
            });
            assert_eq!(count, expected_counts[pos],
                "pos={} expected {} matches, got {}", pos, expected_counts[pos], count);
        }
    });
}

#[test]
fn test_no_matches() {
    run_with_stack(|| {
        let data: &[u8] = b"aa bb cc";
        let em = SubstringEnumerator::new(data, 2, 273);

        for pos in 0..8 {
            em.for_each(pos, |_offset, _length| {
                panic!("Expected no matches at pos={}", pos);
            });
        }
    });
}

fn main() {}
