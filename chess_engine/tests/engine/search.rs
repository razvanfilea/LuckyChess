use crate::common::{board, find_move};
use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::search::{HistoryTable, search};
use chess_engine::time::TimeManager;
use chess_engine::transposition::{TTEntry, TTFlag, TranspositionTable};
use chess_engine::uci::format_move;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Searches with a fresh TT, returning the best move and the info lines
fn run(board: &Board, history: &[u64], time_manager: TimeManager) -> (Move, Vec<String>) {
    let tt = Arc::new(TranspositionTable::with_buckets(16));
    run_with_tt(board, history, time_manager, &tt)
}

fn run_with_tt(
    board: &Board,
    history: &[u64],
    time_manager: TimeManager,
    tt: &Arc<TranspositionTable>,
) -> (Move, Vec<String>) {
    let stop = Arc::new(AtomicBool::new(false));
    let mut info = Vec::new();
    let best = search(board.clone(), history, time_manager, stop, tt, |line| {
        info.push(line)
    });
    (best, info)
}

fn has_score(info: &[String], score: &str) -> bool {
    info.iter().any(|line| line.contains(score))
}

#[test]
fn test_history_table() {
    let mut history = HistoryTable::default();
    assert_eq!(history.get(Color::White, Sq::E2, Sq::E4), 0);

    history.update_bonus(Color::White, Sq::E2, Sq::E4, 3);
    assert!(history.get(Color::White, Sq::E2, Sq::E4) > 0);
    assert_eq!(history.get(Color::Black, Sq::E2, Sq::E4), 0);
    // Gravity keeps scores bounded
    for _ in 0..100 {
        history.update_bonus(Color::White, Sq::E2, Sq::E4, 8);
    }
    assert!((1..=10_000).contains(&history.get(Color::White, Sq::E2, Sq::E4)));

    history.clear();
    assert_eq!(history.get(Color::White, Sq::E2, Sq::E4), 0);

    for _ in 0..100 {
        history.update_malus(Color::White, Sq::E2, Sq::E4, 8);
    }
    assert!((-10_000..0).contains(&history.get(Color::White, Sq::E2, Sq::E4)));
}

#[test]
fn test_search_start_pos() {
    let depth = if cfg!(miri) { 1 } else { 2 };
    let board = Board::start_pos();
    let (best, info) = run(&board, &[], TimeManager::from_depth(depth));

    assert!(board.legal(best));
    assert_eq!(info.len(), depth as usize);
    for (i, line) in info.iter().enumerate() {
        assert!(
            line.starts_with(&format!("info depth {} ", i + 1)),
            "{line}"
        );
        assert!(line.contains("score cp"), "{line}");
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_positions() {
    // (position, depth, best move or "" for none, expected score)
    let cases = [
        // Mate in one
        (
            "r1bqkb1r/pppp1ppp/2n5/4p3/2B1n3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 0 1",
            1,
            "f3f7",
            "score mate 1",
        ),
        (
            "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 1",
            1,
            "d8h4",
            "score mate 1",
        ),
        // Promotion
        ("8/4P3/8/8/8/8/k7/4K3 w - - 0 1", 1, "e7e8q", "score cp"),
        // Checkmated and stalemated: no move
        (
            "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 2",
            1,
            "",
            "score mate",
        ),
        ("k7/8/1Q6/8/8/8/8/K7 b - - 0 1", 1, "", "score cp 0"),
    ];
    for (fen, depth, expected, score) in cases {
        let (best, info) = run(&board(fen), &[], TimeManager::from_depth(depth));
        if expected.is_empty() {
            assert!(best.is_none(), "{fen}: {best:?}");
        } else {
            assert_eq!(format_move(best), expected, "{fen}");
        }
        assert!(has_score(&info, score), "{fen}: {info:?}");
    }

    // Draws score zero
    for fen in [
        "k7/8/8/8/8/8/8/K6R w - - 100 1",
        "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
    ] {
        let (_, info) = run(&board(fen), &[], TimeManager::from_depth(1));
        assert!(has_score(&info, "score cp 0"), "{fen}: {info:?}");
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_avoids_losing_capture() {
    // Nxd4 exd4 loses the knight for a pawn
    let board = board("4k3/8/8/4p3/3p4/2N5/8/4K3 w - - 0 1");
    let (best, _) = run(&board, &[], TimeManager::from_depth(2));
    assert_ne!(format_move(best), "c3d4");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_aspiration_windows() {
    // Aspiration windows start at depth 5
    let board = board("k7/8/K7/8/8/8/7p/7R w - - 0 1");
    let (best, info) = run(&board, &[], TimeManager::from_depth(5));
    assert!(
        ["a6b6", "h1h2"].contains(&format_move(best).as_str()),
        "{best:?}"
    );
    assert_eq!(info.len(), 5);
    assert!(info[4].starts_with("info depth 5"));

    // An inflated depth 4 score forces depth 5 to fail low and widen
    let tt = Arc::new(TranspositionTable::with_buckets(16));
    let entry = TTEntry::new(Move::NONE, 2000, 100, 4, TTFlag::LowerBound, false);
    tt.store(board.hash(), entry, 0);
    let (best, _) = run_with_tt(&board, &[], TimeManager::from_depth(5), &tt);
    assert!(
        ["a6b6", "h1h2"].contains(&format_move(best).as_str()),
        "{best:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_stop() {
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    // Stopped before starting: no depth completes and the TT stays empty
    let stop = Arc::new(AtomicBool::new(true));
    let mut lines = 0;
    search(
        Board::start_pos(),
        &[],
        TimeManager::from_depth(10),
        stop,
        &tt,
        |_| lines += 1,
    );
    assert_eq!(lines, 0);
    assert_eq!(tt.hashfull(), 0);

    // Stopped after depth 1
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let mut lines = 0;
    search(
        Board::start_pos(),
        &[],
        TimeManager::from_depth(10),
        stop,
        &tt,
        |_| {
            lines += 1;
            stop_clone.store(true, Ordering::Relaxed);
        },
    );
    assert_eq!(lines, 1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_ponder_move() {
    let board = board("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let tt = Arc::new(TranspositionTable::with_buckets(16));
    let (best, _) = run_with_tt(&board, &[], TimeManager::from_depth(3), &tt);
    assert!(board.legal(best));

    let mut next = board.clone();
    next.make_move(best);
    if let Some(entry) = tt.probe(next.hash(), 1)
        && !entry.mov.is_none()
    {
        assert!(next.legal(entry.mov));
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_repetition_before_root() {
    // White is a queen up; Black shuffles the knight to repeat the starting position
    let play = |moves: &str| {
        let mut board = board("7k/3n4/8/8/8/8/Q7/7K w - - 0 1");
        let mut history = vec![board.hash()];
        for uci in moves.split_whitespace() {
            board.make_move(find_move(&board, uci));
            history.push(board.hash());
        }
        (board, history)
    };

    // b8d7 repeats the position a third time
    let (board, history) = play("h1h2 d7b8 h2h1 b8d7 h1h2 d7b8 h2h1");
    let (best, info) = run(&board, &history, TimeManager::from_depth(2));
    assert_eq!(format_move(best), "b8d7");
    assert!(has_score(&info, "score cp 0"), "{info:?}");

    // Only a second occurrence: not a draw
    let (board, history) = play("h1h2 d7b8 h2h1");
    let (_, info) = run(&board, &history, TimeManager::from_depth(2));
    assert!(!has_score(&info, "score cp 0"), "{info:?}");
}
