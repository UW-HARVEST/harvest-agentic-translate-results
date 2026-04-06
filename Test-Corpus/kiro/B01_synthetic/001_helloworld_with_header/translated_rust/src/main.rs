mod sillymain;

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(sillymain::helloworld() as u8)
}
