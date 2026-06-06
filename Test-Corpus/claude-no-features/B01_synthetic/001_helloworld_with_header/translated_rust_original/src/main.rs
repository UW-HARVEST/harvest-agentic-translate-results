mod sillymain;

use std::process::ExitCode;

fn main() -> ExitCode {
    let code = sillymain::helloworld();
    ExitCode::from(code as u8)
}
