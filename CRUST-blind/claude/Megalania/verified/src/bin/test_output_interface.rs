use Megalania::output_interface::OutputInterface;

struct StringOut {
    data: Vec<u8>,
}
impl OutputInterface for StringOut {
    fn write(&mut self, data: &[u8]) -> bool {
        self.data.extend_from_slice(data);
        true
    }
}

#[test]
fn test_output_trait() {
    let mut out = StringOut { data: vec![] };
    assert!(out.write(b"hello"));
    assert!(out.write(b" world"));
    assert_eq!(out.data, b"hello world");
}

#[test]
fn test_output_trait_object() {
    let mut out = StringOut { data: vec![] };
    let dyn_out: &mut dyn OutputInterface = &mut out;
    assert!(dyn_out.write(&[1, 2, 3]));
    assert_eq!(out.data, vec![1, 2, 3]);
}

fn main() {}
