use std::io::Write;
fn main(){ std::thread::sleep(std::time::Duration::from_millis(300));
  let mut o=std::io::stdout(); let _=write!(o,"{}\n",12345); let _=o.flush();
  std::process::exit(0); }
