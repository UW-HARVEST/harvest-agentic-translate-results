fn main(){let mut a=[0u8;8]; a[..3].copy_from_slice(b"abc"); let s=String::from_utf8_lossy(&a); println!("{:?} len={}", s, s.len()); println!("eq={}", s=="abc");}
