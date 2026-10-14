#![cfg(not(miri))]

use crate::common::START_FEN;
use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::time::{SearchOptions, TimeManager};
use chess_engine::uci::{UciState, format_move, parse_go};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn go_startpos(args: &str) -> SearchOptions {
    parse_go(args, &Board::start_pos())
}

struct Harness {
    uci: UciState,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Harness {
    fn new() -> Self {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        let uci = UciState::new(move |line| sink.lock().unwrap().push(line));
        Self { uci, lines }
    }

    fn run(&mut self, command: &str) -> String {
        self.lines.lock().unwrap().clear();
        assert!(self.uci.process_command(command));
        self.lines.lock().unwrap().join("\n")
    }

    fn fen(&mut self) -> String {
        let out = self.run("d");
        let line = out.lines().find_map(|l| l.strip_prefix("FEN: "));
        line.expect("`d` prints the FEN").to_string()
    }
}

#[test]
fn test_uci_handshake() {
    let mut h = Harness::new();
    let out = h.run("uci");
    assert!(out.starts_with("id name Lucky Chess"));
    assert!(out.ends_with("uciok"));
    assert_eq!(h.run("isready"), "readyok");
}

#[test]
fn test_empty_and_unknown_commands_are_ignored() {
    let mut h = Harness::new();
    assert_eq!(h.run(""), "");
    assert_eq!(h.run("   \t"), "");
    assert_eq!(h.run("foo bar"), "");
    assert_eq!(h.fen(), START_FEN);
}

#[test]
fn test_commands_are_case_insensitive() {
    let mut h = Harness::new();
    assert!(h.run("D").contains("FEN: "));
    assert!(h.run("EVAL").starts_with("score: cp "));
    assert_eq!(h.run("IsReady"), "readyok");
}

#[test]
fn test_quit() {
    let mut h = Harness::new();
    assert!(!h.uci.process_command("quit"));
}

#[test]
fn test_position_startpos() {
    let mut h = Harness::new();
    h.run("position startpos moves e2e4 e7e5 g1f3");
    assert_eq!(
        h.fen(),
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2"
    );

    h.run("position startpos");
    assert_eq!(h.fen(), START_FEN);

    h.run("position STARTPOS MOVES e2e4");
    assert_eq!(
        h.fen(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
    );
}

#[test]
fn test_position_fen() {
    let mut h = Harness::new();
    h.run("position fen 4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1 moves e1g1");
    assert_eq!(h.fen(), "4k3/8/8/8/8/8/8/R4RK1 b - - 1 1");

    h.run("position fen 4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert_eq!(h.fen(), "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
}

#[test]
fn test_position_promotions() {
    let mut h = Harness::new();
    let fen = "8/P6k/8/8/8/8/8/K7 w - - 0 1";

    h.run(&format!("position fen {fen} moves a7a8n"));
    assert_eq!(h.fen(), "N7/7k/8/8/8/8/8/K7 b - - 0 1");

    h.run(&format!("position fen {fen} moves A7A8Q"));
    assert_eq!(h.fen(), "Q7/7k/8/8/8/8/8/K7 b - - 0 1");

    h.run(&format!("position fen {fen} moves a7a8"));
    assert_eq!(h.fen(), fen);
}

#[test]
fn test_position_stops_at_bad_move() {
    let mut h = Harness::new();
    let after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";

    for bad in [
        "e2e4", "e7e8q", "0000", "e2", "i1i2", "e7e5k", "eé2e", "e7é5",
    ] {
        h.run(&format!("position startpos moves e2e4 {bad} d7d5"));
        assert_eq!(h.fen(), after_e4, "bad move: {bad}");
    }
}

#[test]
fn test_invalid_position_keeps_board() {
    let mut h = Harness::new();
    h.run("position startpos moves e2e4");
    let fen = h.fen();

    for bad in [
        "",
        "fen",
        "fen not a fen",
        "fen 8/8/8/8/8/8/8/8 w - - 0 1",
        "fen 4k3/8/8/8/8/8/8/8 w - - 0 1",
        "fen 4k3/8/8/8/8/8/8/K3K3 w - - 0 1",
        "startfen",
        "moves e2e4",
    ] {
        h.run(&format!("position {bad}"));
        assert_eq!(h.fen(), fen, "bad position: {bad}");
    }
}

#[test]
fn test_setoption() {
    let mut h = Harness::new();
    h.run("setoption name Move Overhead value 25");
    assert_eq!(h.uci.move_overhead(), 25);
    h.run("setoption name moveoverhead value 40");
    assert_eq!(h.uci.move_overhead(), 40);
    h.run("SETOPTION NAME MOVE OVERHEAD VALUE 50");
    assert_eq!(h.uci.move_overhead(), 50);
    h.run("setoption name Move Overhead value 999999");
    assert_eq!(h.uci.move_overhead(), 5000);

    h.run("setoption name Move Overhead value -3");
    h.run("setoption name Move Overhead");
    h.run("setoption Move Overhead value 7");
    assert_eq!(h.uci.move_overhead(), 5000);

    h.run("setoption name Hash value 0");
    h.run("setoption name Hash value 1");
    h.run("setoption name ClearHash");
    h.run("setoption name Threads value 1");
}

#[test]
fn test_perft() {
    let mut h = Harness::new();
    assert_eq!(h.run("perft 3"), "Nodes searched: 8902");
    assert_eq!(h.run("go perft 2"), "Nodes searched: 400");
    assert_eq!(h.run("GO PERFT 2"), "Nodes searched: 400");
    h.run("position startpos moves e2e4");
    assert_eq!(h.run("go perft 1"), "Nodes searched: 20");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_go_depth_prints_bestmove() {
    let mut h = Harness::new();
    h.run("position startpos moves e2e4 e7e5");
    h.run("go depth 4");
    let out = h.run("wait");
    let best = out.lines().last().expect("search prints output");
    assert!(best.starts_with("bestmove "), "{best}");
}

#[test]
fn test_parse_go_without_limits_is_infinite() {
    for args in ["", "  ", "infinite", "ponder", "winc 10 binc 10", "foo 3"] {
        assert!(go_startpos(args).infinite, "{args:?}");
    }
    for args in [
        "depth 5",
        "nodes 100",
        "movetime 10",
        "wtime 1000",
        "btime 1000",
    ] {
        assert!(!go_startpos(args).infinite, "{args:?}");
    }
}

#[test]
fn test_parse_go_is_case_insensitive() {
    let opts = go_startpos("WTIME 1000 Depth 5");
    assert_eq!(opts.wtime, Some(Duration::from_millis(1000)));
    assert_eq!(opts.depth, Some(5));
}

#[test]
fn test_parse_go_clock() {
    let opts = go_startpos("wtime 1000 btime 2000 winc 10 binc 20 movestogo 30");
    assert_eq!(opts.wtime, Some(Duration::from_millis(1000)));
    assert_eq!(opts.btime, Some(Duration::from_millis(2000)));
    assert_eq!(opts.winc, Some(Duration::from_millis(10)));
    assert_eq!(opts.binc, Some(Duration::from_millis(20)));
    assert_eq!(opts.movestogo, Some(30));
}

#[test]
fn test_parse_go_clamps_numbers() {
    let opts = go_startpos("wtime -50 winc -10 depth 99999999999999999999999 nodes abc");
    assert_eq!(opts.wtime, Some(Duration::ZERO));
    assert_eq!(opts.winc, Some(Duration::ZERO));
    assert_eq!(opts.depth, Some(u64::MAX));
    assert_eq!(opts.nodes, None);

    let tm = TimeManager::from_options(&go_startpos("depth 300"), Color::White, 0);
    assert_eq!(tm.limits.max_depth, 64);
    let tm = TimeManager::from_options(&go_startpos("depth 0"), Color::White, 0);
    assert_eq!(tm.limits.max_depth, 1);
}

#[test]
fn test_parse_go_skips_unused_tokens() {
    let opts = go_startpos("searchmoves e2e4 d2d4 ponder mate 3 depth 10 nodes 50000 movetime 100");
    assert_eq!(opts.depth, Some(10));
    assert_eq!(opts.nodes, Some(50000));
    assert_eq!(opts.movetime, Some(Duration::from_millis(100)));
    assert!(!opts.infinite);
}

#[test]
fn test_parse_go_searchmoves() {
    let names = |opts: &SearchOptions| {
        opts.searchmoves
            .iter()
            .map(|&m| format_move(m))
            .collect::<Vec<_>>()
    };

    let opts = go_startpos("searchmoves e2e4 G1F3 depth 7");
    assert_eq!(names(&opts), ["e2e4", "g1f3"]);
    assert_eq!(opts.depth, Some(7));

    // Stops at the first illegal move; the rest are skipped as unknown tokens
    let opts = go_startpos("searchmoves d2d4 e2e5 c2c4 wtime 100");
    assert_eq!(names(&opts), ["d2d4"]);
    assert_eq!(opts.wtime, Some(Duration::from_millis(100)));

    assert!(go_startpos("searchmoves").searchmoves.is_empty());
    assert!(go_startpos("depth 3").searchmoves.is_empty());

    // Moves are checked against the given position
    let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 b - - 0 1").unwrap();
    let opts = parse_go("searchmoves e2e4 e8d8", &board);
    assert!(opts.searchmoves.is_empty());
    let opts = parse_go("searchmoves e8d8 e2e4", &board);
    assert_eq!(names(&opts), ["e8d8"]);
}
