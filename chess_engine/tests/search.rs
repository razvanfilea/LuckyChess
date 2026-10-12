use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::move_gen::gen_all_moves;
use chess_engine::nnue::{FinnyTable, evaluate};

/// A Finny table whose entries have already seen `board`.
fn primed_table(board: &Board) -> FinnyTable {
    let mut table = FinnyTable::default();
    table.eval(board);
    table
}
use chess_engine::search::{HistoryTable, search as engine_search};
use chess_engine::time::TimeManager;
use chess_engine::transposition::{TTEntry, TTFlag, TranspositionTable};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn search(
    board: Board,
    time_manager: TimeManager,
    stop_requested: Arc<AtomicBool>,
    tt: &Arc<TranspositionTable>,
    on_info: impl FnMut(String),
) -> Move {
    engine_search(board, &[], time_manager, stop_requested, tt, on_info)
}

#[test]
fn test_finny_single_move_parity() {
    let board = Board::start_pos();

    let moves = gen_all_moves(&board);
    let limit = if cfg!(miri) { 1 } else { usize::MAX };
    let mut tested = 0;
    for &scored in moves.as_slice() {
        let mov = scored.mov;
        if !board.legal(mov) {
            continue;
        }

        let mut child = board.clone();
        child.make_move(mov);

        let mut table = primed_table(&board);
        let actual = table.eval(&child);

        let expected = evaluate(&child);

        assert_eq!(actual, expected, "mismatch for move {mov:?}");
        tested += 1;
        if tested >= limit {
            break;
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_finny_two_move_parity() {
    let board = Board::start_pos();

    // Try c2c3 then every legal Black response
    let mov1 = Move::new(Sq::C2, Sq::C3, MoveFlags::Quiet);
    let mut b1 = board.clone();
    b1.make_move(mov1);

    let moves2 = gen_all_moves(&b1);
    let limit = if cfg!(miri) { 1 } else { usize::MAX };
    let mut tested = 0;
    for &scored in moves2.as_slice() {
        let mov2 = scored.mov;
        if !b1.legal(mov2) {
            continue;
        }

        let mut b2 = b1.clone();
        b2.make_move(mov2);

        let mut table = primed_table(&board);
        let _ = table.eval(&b1);
        let actual = table.eval(&b2);

        let expected = evaluate(&b2);

        assert_eq!(
            actual,
            expected,
            "2-ply mismatch: c2c3 then {mov2:?} (board: {})",
            b2.to_fen()
        );
        tested += 1;
        if tested >= limit {
            break;
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_finny_tree_parity() {
    fn dfs(board: &mut Board, table: &mut FinnyTable, depth: usize) {
        if depth == 0 {
            return;
        }
        let moves = gen_all_moves(board);
        for &scored in moves.as_slice() {
            let mov = scored.mov;
            if !board.legal(mov) {
                continue;
            }
            board.make_move(mov);
            let actual = table.eval(board);
            let expected = evaluate(board);
            assert_eq!(
                actual,
                expected,
                "Tree mismatch at FEN: {}, move: {:?}",
                board.to_fen(),
                mov
            );
            dfs(board, table, depth - 1);
            board.undo_move(mov);
        }
    }

    let mut board =
        Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap();
    let mut table = primed_table(&board);
    dfs(&mut board, &mut table, 2);
}

#[test]
fn test_history_table_operations() {
    let mut history = HistoryTable::default();

    // Verify initial values are zero
    assert_eq!(history.get(Color::White, Sq::E2, Sq::E4), 0);
    assert_eq!(history.get(Color::Black, Sq::E7, Sq::E5), 0);

    // Apply updates
    history.update_bonus(Color::White, Sq::E2, Sq::E4, 3);
    let val1 = history.get(Color::White, Sq::E2, Sq::E4);
    assert!(val1 > 0);

    // Apply multiple updates and verify gravity damping (stays bounded <= 10_000)
    for _ in 0..100 {
        history.update_bonus(Color::White, Sq::E2, Sq::E4, 8);
    }
    let bounded_val = history.get(Color::White, Sq::E2, Sq::E4);
    assert!(bounded_val <= 10_000);
    assert!(bounded_val > 0);

    // Verify clear resets everything
    history.clear();
    assert_eq!(history.get(Color::White, Sq::E2, Sq::E4), 0);

    // Test malus updates
    history.update_malus(Color::White, Sq::E2, Sq::E4, 3);
    let val_malus = history.get(Color::White, Sq::E2, Sq::E4);
    assert!(val_malus < 0);

    // Repeated malus updates stay bounded >= -10_000 without integer underflow
    for _ in 0..100 {
        history.update_malus(Color::White, Sq::E2, Sq::E4, 8);
    }
    let bounded_malus = history.get(Color::White, Sq::E2, Sq::E4);
    assert!(bounded_malus >= -10_000);
    assert!(bounded_malus < 0);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_start_pos_depth_1_and_2() {
    let board = Board::start_pos();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let depth = if cfg!(miri) { 1 } else { 2 };
    let mut info_lines = Vec::new();
    let best_move = search(
        board.clone(),
        TimeManager::from_depth(depth),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(!best_move.is_none());
    assert!(board.legal(best_move));
    assert_eq!(info_lines.len(), depth as usize);
    assert!(info_lines[0].starts_with("info depth 1"));
    if depth >= 2 {
        assert!(info_lines[1].starts_with("info depth 2"));
        assert!(info_lines[1].contains("score cp"));
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_mate_in_1_white_scholars() {
    // Scholar's mate: Qxf7#
    let board =
        Board::from_fen("r1bqkb1r/pppp1ppp/2n5/4p3/2B1n3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 0 1")
            .unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = search(
        board.clone(),
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert_eq!(best_move.from(), Sq::F3);
    assert_eq!(best_move.to(), Sq::F7);
    assert!(best_move.is_capture());
    assert!(info_lines.iter().any(|line| line.contains("score mate 1")));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_mate_in_1_black_fools() {
    // Black plays Qh4#
    let board =
        Board::from_fen("rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = search(
        board.clone(),
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert_eq!(best_move.from(), Sq::D8);
    assert_eq!(best_move.to(), Sq::H4);
    assert!(info_lines.iter().any(|line| line.contains("score mate 1")));
}

#[test]
fn test_search_already_checkmated_terminal() {
    // White is checkmated by Qh4# (0 legal moves and in check)
    let board =
        Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 2").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = search(
        board,
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(best_move.is_none());
    assert!(info_lines.iter().any(|line| line.contains("score mate")));
}

#[test]
fn test_search_stalemate_terminal() {
    // Black King on a8, White Queen on b6, White King on a1. Black to move, 0 legal moves, not in check.
    let board = Board::from_fen("k7/8/1Q6/8/8/8/8/K7 b - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = search(
        board,
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(best_move.is_none());
    assert!(info_lines.iter().any(|line| line.contains("score cp 0")));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_draw_fifty_move_rule() {
    // 50-move rule triggered (halfmove clock = 100)
    let board = Board::from_fen("k7/8/8/8/8/8/8/K6R w - - 100 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let _best_move = search(
        board,
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(info_lines.iter().any(|line| line.contains("score cp 0")));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_draw_insufficient_material() {
    // King vs King
    let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let _best_move = search(
        board,
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(info_lines.iter().any(|line| line.contains("score cp 0")));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_stop_requested_before_search() {
    let board = Board::start_pos();
    let stop_requested = Arc::new(AtomicBool::new(true)); // Already stopped
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut count = 0;
    let _ = search(
        board,
        TimeManager::from_depth(5),
        stop_requested,
        &tt,
        |_| count += 1,
    );

    // Since stop was set immediately, 0 depths should complete
    assert_eq!(count, 0);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_stop_requested_during_search() {
    let board = Board::start_pos();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let stop_clone = Arc::clone(&stop_requested);
    let completed_depths = Arc::new(AtomicUsize::new(0));
    let completed_clone = Arc::clone(&completed_depths);

    let _ = search(
        board,
        TimeManager::from_depth(10),
        stop_requested,
        &tt,
        move |_line| {
            completed_clone.fetch_add(1, Ordering::Relaxed);
            // Abort after depth 1
            stop_clone.store(true, Ordering::Relaxed);
        },
    );

    assert_eq!(completed_depths.load(Ordering::Relaxed), 1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_qsearch_avoids_losing_capture() {
    // White Knight on c3 can capture the pawn on d4, but Black recaptures exd4.
    // Nxd4? exd4 loses a knight (1350) for a pawn (400). The engine should not
    // play Nxd4 because qsearch resolves the recapture.
    let board = Board::from_fen("4k3/8/8/4p3/3p4/2N5/8/4K3 w - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let best_move = search(
        board,
        TimeManager::from_depth(2),
        stop_requested,
        &tt,
        |_| {},
    );

    let nxd4 = Move::new(Sq::C3, Sq::D4, MoveFlags::Capture);
    assert_ne!(
        best_move, nxd4,
        "Engine should avoid Nxd4 (loses knight for pawn after exd4)"
    );
}

#[test]
fn test_search_qsearch_promotes_pawn() {
    let board = Board::from_fen("8/4P3/8/8/8/8/k7/4K3 w - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));
    let promo_mov = search(
        board,
        TimeManager::from_depth(1),
        stop_requested,
        &tt,
        |_| {},
    );
    assert_eq!(promo_mov.from(), Sq::E7);
    assert_eq!(promo_mov.to(), Sq::E8);
    assert!(promo_mov.is_promotion());
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_aspiration_window_depth_5() {
    // Search at depth 5 activates aspiration windows (ASPIRATION_MIN_DEPTH = 5).
    let board = Board::from_fen("k7/8/K7/8/8/8/7p/7R w - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = search(
        board,
        TimeManager::from_depth(5),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    assert!(
        (best_move.from() == Sq::A6 && best_move.to() == Sq::B6)
            || (best_move.from() == Sq::H1 && best_move.to() == Sq::H2),
        "expected a6b6 or h1h2, got {best_move:?}"
    );
    assert_eq!(info_lines.len(), 5);
    assert!(info_lines[4].starts_with("info depth 5"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_aspiration_fail_low_widening() {
    // Force fail-low in aspiration search by pre-seeding high score at root
    let board = Board::from_fen("k7/8/K7/8/8/8/7p/7R w - - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    // Seed depth 4 with artificially high score (+2000) so depth 5 fails low initially
    let entry = TTEntry::new(Move::NONE, 2000, 100, 4, TTFlag::LowerBound, false);
    tt.store(board.hash(), entry, 0);

    let mut info_lines = Vec::new();
    let best_move = search(
        board,
        TimeManager::from_depth(5),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );
    assert!(
        (best_move.from() == Sq::A6 && best_move.to() == Sq::B6)
            || (best_move.from() == Sq::H1 && best_move.to() == Sq::H2),
        "expected a6b6 or h1h2, got {best_move:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_ponder_move() {
    let board =
        Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let best_move = search(
        board.clone(),
        TimeManager::from_depth(if cfg!(miri) { 1 } else { 3 }),
        stop_requested,
        &tt,
        |_| {},
    );

    assert!(!best_move.is_none());
    assert!(board.legal(best_move));

    let mut next_board = board.clone();
    next_board.make_move(best_move);
    if let Some(entry) = tt.probe(next_board.hash(), 1)
        && entry.mov != Move::NONE
    {
        assert!(next_board.legal(entry.mov));
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_pre_root_threefold_repetition() {
    // Position A: White Kh1, Qa2; Black Kh8, Nd7. White to move.
    // Queen on a2 does not attack h8, b8, or d7.
    let mut board = Board::from_fen("7k/3n4/8/8/8/8/Q7/7K w - - 0 1").unwrap();
    let mut history = Vec::new();

    // Position A (occurrence 1, index 0)
    history.push(board.hash());

    let w_to_h2 = Move::new(Sq::H1, Sq::H2, MoveFlags::Quiet);
    let w_to_h1 = Move::new(Sq::H2, Sq::H1, MoveFlags::Quiet);
    let b_to_b8 = Move::new(Sq::D7, Sq::B8, MoveFlags::Quiet);
    let b_to_d7 = Move::new(Sq::B8, Sq::D7, MoveFlags::Quiet);

    // 1. Kh2 Nb8
    board.make_move(w_to_h2);
    history.push(board.hash()); // index 1
    board.make_move(b_to_b8);
    history.push(board.hash()); // index 2

    // 2. Kh1 Nd7 -> Position A (occurrence 2, index 4)
    board.make_move(w_to_h1);
    history.push(board.hash()); // index 3
    board.make_move(b_to_d7);
    history.push(board.hash()); // index 4

    assert_eq!(history[0], history[4]);

    // 3. Kh2 Nb8
    board.make_move(w_to_h2);
    history.push(board.hash()); // index 5
    board.make_move(b_to_b8);
    history.push(board.hash()); // index 6

    // 4. Kh1 (White played Kh1, now Black to move, index 7)
    board.make_move(w_to_h1);
    history.push(board.hash()); // index 7

    // Now Black is to move. Black is down a full Queen.
    // If Black plays 4... Nd7, Position A appears for the 3rd time in the game (3-fold draw!).
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let best_move = engine_search(
        board.clone(),
        &history,
        TimeManager::from_depth(if cfg!(miri) { 1 } else { 2 }),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    // Black must choose Nd7 to claim the 3-fold repetition draw!
    assert_eq!(best_move, b_to_d7);
    // Score must be draw (cp 0)
    assert!(info_lines.iter().any(|line| line.contains("score cp 0")));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_pre_root_twofold_repetition_not_draw() {
    // Position A: White Kh1, Qa2; Black Kh8, Nd7. White to move.
    let mut board = Board::from_fen("7k/3n4/8/8/8/8/Q7/7K w - - 0 1").unwrap();
    let mut history = Vec::new();

    // Position A (occurrence 1, index 0)
    history.push(board.hash());

    let w_to_h2 = Move::new(Sq::H1, Sq::H2, MoveFlags::Quiet);
    let w_to_h1 = Move::new(Sq::H2, Sq::H1, MoveFlags::Quiet);
    let b_to_b8 = Move::new(Sq::D7, Sq::B8, MoveFlags::Quiet);
    let _b_to_d7 = Move::new(Sq::B8, Sq::D7, MoveFlags::Quiet);

    // 1. Kh2 Nb8
    board.make_move(w_to_h2);
    history.push(board.hash()); // index 1
    board.make_move(b_to_b8);
    history.push(board.hash()); // index 2

    // 2. Kh1 (White played Kh1, now Black to move, index 3)
    board.make_move(w_to_h1);
    history.push(board.hash()); // index 3

    // In history, Position A has only occurred ONCE (at index 0).
    // If Black plays 2... Nd7, Position A appears for the 2nd time (NOT 3-fold repetition).
    // Therefore, Black cannot claim a draw, and the eval should NOT be cp 0.
    let stop_requested = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TranspositionTable::with_buckets(16));

    let mut info_lines = Vec::new();
    let _best_move = engine_search(
        board.clone(),
        &history,
        TimeManager::from_depth(if cfg!(miri) { 1 } else { 2 }),
        stop_requested,
        &tt,
        |info| info_lines.push(info),
    );

    // Since Black is down a full queen and cannot claim 3-fold draw yet, score is negative
    assert!(!info_lines.iter().any(|line| line.contains("score cp 0")));
}
