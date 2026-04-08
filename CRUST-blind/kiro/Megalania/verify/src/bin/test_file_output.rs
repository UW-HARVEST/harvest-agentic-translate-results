use Megalania::file_output::FileOutput;
use Megalania::output_interface::OutputInterface;
use std::io::Read;

#[test]
fn test_file_output_write() {
    let tmp = std::env::temp_dir().join("megalania_test_file_output.bin");
    let file = std::fs::File::create(&tmp).unwrap();
    let mut fo = FileOutput::from_file(file);
    assert!(fo.write(&[0x41, 0x42, 0x43]));
    drop(fo);

    let mut buf = Vec::new();
    std::fs::File::open(&tmp).unwrap().read_to_end(&mut buf).unwrap();
    assert_eq!(buf, vec![0x41, 0x42, 0x43]);
    std::fs::remove_file(&tmp).ok();
}

fn main() {}
