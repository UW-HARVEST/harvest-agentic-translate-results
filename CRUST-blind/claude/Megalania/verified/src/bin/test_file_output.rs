use std::fs::File;
use Megalania::file_output::FileOutput;
use Megalania::output_interface::OutputInterface;

#[test]
fn test_file_output_writes() {
    let path = std::env::temp_dir().join("test_file_output_writes.bin");
    let _ = std::fs::remove_file(&path);
    {
        let f = File::create(&path).expect("could not create file");
        let mut fo = FileOutput { file: f };
        let payload = b"hello world";
        let ok = fo.write(payload);
        assert!(ok);
        // Drop fo to flush.
    }
    let contents = std::fs::read(&path).expect("could not read back");
    assert_eq!(contents, b"hello world");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_file_output_multiple_writes() {
    let path = std::env::temp_dir().join("test_file_output_multi.bin");
    let _ = std::fs::remove_file(&path);
    {
        let f = File::create(&path).expect("could not create file");
        let mut fo = FileOutput { file: f };
        assert!(fo.write(&[1, 2, 3]));
        assert!(fo.write(&[4, 5]));
        assert!(fo.write(&[6, 7, 8, 9]));
    }
    let contents = std::fs::read(&path).expect("could not read back");
    assert_eq!(contents, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
