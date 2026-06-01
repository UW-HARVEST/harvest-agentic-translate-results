use Megalania::output_interface::OutputInterface;

struct CollectorOut {
    data: Vec<u8>,
    succeeds: bool,
}

impl OutputInterface for CollectorOut {
    fn write(&mut self, data: &[u8]) -> bool {
        self.data.extend_from_slice(data);
        self.succeeds
    }
}

#[test]
fn test_output_interface_collects_data() {
    let mut out = CollectorOut {
        data: Vec::new(),
        succeeds: true,
    };
    assert!(out.write(&[1, 2, 3]));
    assert!(out.write(&[4, 5]));
    assert_eq!(out.data, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_output_interface_failure_propagates() {
    let mut out = CollectorOut {
        data: Vec::new(),
        succeeds: false,
    };
    assert!(!out.write(&[1, 2, 3]));
}

fn main() {}
