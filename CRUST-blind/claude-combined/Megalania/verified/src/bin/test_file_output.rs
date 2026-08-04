use Megalania::file_output::FileOutput;
use Megalania::output_interface::OutputInterface;
use std::fs::OpenOptions;
use std::io::Read;

#[test]
fn test_file_output_writes() {
    // Open a temp file for writing
    let mut path = std::env::temp_dir();
    path.push(format!("megalania_fo_test_{}", std::process::id()));
    {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("open");
        let mut fo = FileOutput::new(f);
        assert!(fo.write(&[1, 2, 3]));
        assert!(fo.write(&[4, 5]));
    }
    // Read back and verify
    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("open ro")
        .read_to_end(&mut buf)
        .expect("read");
    assert_eq!(buf, vec![1, 2, 3, 4, 5]);
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_file_output_empty_write() {
    let mut path = std::env::temp_dir();
    path.push(format!("megalania_fo_test2_{}", std::process::id()));
    {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("open");
        let mut fo = FileOutput::new(f);
        assert!(fo.write(&[]));
    }
    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("open ro")
        .read_to_end(&mut buf)
        .expect("read");
    assert_eq!(buf, Vec::<u8>::new());
    std::fs::remove_file(&path).ok();
}

fn main() {}
