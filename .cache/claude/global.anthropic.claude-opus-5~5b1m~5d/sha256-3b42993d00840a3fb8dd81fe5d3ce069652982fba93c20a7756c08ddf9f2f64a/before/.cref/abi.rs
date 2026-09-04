#[repr(C, align(4))] struct ConfigFlags { storage: [u8;4] }
#[repr(C)] struct ProcessState { flags: ConfigFlags, base_value: i32, multiplier: i32, operation: i8 }
fn main(){
  println!("RS ConfigFlags  size={} align={}", std::mem::size_of::<ConfigFlags>(), std::mem::align_of::<ConfigFlags>());
  println!("RS ProcessState size={} align={}", std::mem::size_of::<ProcessState>(), std::mem::align_of::<ProcessState>());
  println!("RS offsets: flags={} base={} mult={} op={}",
    std::mem::offset_of!(ProcessState,flags), std::mem::offset_of!(ProcessState,base_value),
    std::mem::offset_of!(ProcessState,multiplier), std::mem::offset_of!(ProcessState,operation));
}
