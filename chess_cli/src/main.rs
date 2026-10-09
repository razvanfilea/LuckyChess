mod bench;
mod perft;

use chess_engine::uci::UciState;
use std::io::{self, BufRead};

fn handle_cli_command(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or("");

    if command.eq_ignore_ascii_case("bench") {
        let depth = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(10);
        bench::run_bench(depth, 16, |line| println!("{line}"));
        return true;
    }

    if command.eq_ignore_ascii_case("perft-suite") {
        let depth = parts.next().and_then(|s| s.parse::<u8>().ok());
        perft::run_perft_suite(depth);
        return true;
    }

    if command.eq_ignore_ascii_case("perft")
        && parts
            .clone()
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("suite"))
    {
        let _ = parts.next();
        let depth = parts.next().and_then(|s| s.parse::<u8>().ok());
        perft::run_perft_suite(depth);
        return true;
    }

    false
}

fn main() {
    let mut uci = UciState::new(|line| println!("{line}"));

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let cmd = args[1..].join(" ");
        if handle_cli_command(&cmd) {
            return;
        }
        uci.process_command(&cmd);
        return;
    }

    let stdin = io::stdin();
    let mut input_string = String::new();
    while stdin.lock().read_line(&mut input_string).unwrap_or(0) > 0 {
        if !handle_cli_command(input_string.trim()) && !uci.process_command(&input_string) {
            break;
        }
        input_string.clear();
    }
}
