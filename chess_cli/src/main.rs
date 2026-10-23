#[cfg(lucky_universal)]
mod universal;

fn main() {
    #[cfg(lucky_universal)]
    universal::main();

    #[cfg(not(lucky_universal))]
    chess_cli::run(std::env::args().collect());
}
