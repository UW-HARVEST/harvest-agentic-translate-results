use std::io::Write;
fn restore_default_sigpipe(){ extern "C"{ fn signal(s:i32,h:usize)->usize; } unsafe{ signal(13,0); } }
fn main(){ restore_default_sigpipe();
  std::thread::sleep(std::time::Duration::from_millis(300));
  let mut o=std::io::stdout(); let _=write!(o,"{}\n",12345); let _=o.flush();
  std::process::exit(0); }
