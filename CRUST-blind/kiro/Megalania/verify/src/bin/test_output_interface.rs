use Megalania::output_interface::OutputInterface;

struct MockOutput {
    data: Vec<u8>,
}

impl OutputInterface for MockOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.data.extend_from_slice(data);
        true
    }
}

#[test]
fn test_output_interface_trait() {
    let mut out = MockOutput { data: Vec::new() };
    assert!(out.write(&[1, 2, 3]));
    assert_eq!(out.data, vec![1, 2, 3]);
    assert!(out.write(&[4, 5]));
    assert_eq!(out.data, vec![1, 2, 3, 4, 5]);
}

fn main() {}
