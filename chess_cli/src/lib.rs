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

pub fn run(args: Vec<String>) {
    chess_engine::nnue_advise_huge_pages();
    let mut uci = UciState::new(|line| println!("{line}"));

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

/// Entry point of one engine copy in the universal binary, see `scripts/build_universal.sh`.
///
/// Each copy carries its own `std`, whose runtime is never initialised by `lang_start`,
/// so the arguments are passed in explicitly and stdout is flushed before returning.
///
/// # Safety
/// `argv` must point to `argc` valid NUL-terminated strings
#[cfg(lucky_external_network)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lucky_slice_main(
    argc: std::ffi::c_int,
    argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    use std::io::Write;

    let args = (0..argc as usize)
        // SAFETY: guaranteed by the caller
        .map(|i| unsafe { std::ffi::CStr::from_ptr(*argv.add(i)) })
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    let result = std::panic::catch_unwind(|| run(args));
    let _ = io::stdout().flush();

    if result.is_ok() { 0 } else { 101 }
}
