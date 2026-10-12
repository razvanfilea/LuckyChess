use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::move_gen::{Black, Evasions, NonEvasions, White, gen_moves};

fn test_make_undo_for_fen(fen: &str) {
    let board = Board::from_fen(fen).unwrap();
    let moves = if board.checkers() != 0 {
        if board.to_play == Color::White {
            gen_moves::<White, Evasions>(&board)
        } else {
            gen_moves::<Black, Evasions>(&board)
        }
    } else {
        if board.to_play == Color::White {
            gen_moves::<White, NonEvasions>(&board)
        } else {
            gen_moves::<Black, NonEvasions>(&board)
        }
    };

    let mut tested = 0;
    for &scored_move in moves.as_slice() {
        let mov = scored_move.mov;
        let mut child = board.clone();
        if !child.legal(mov) {
            continue;
        }
        let before = (board.to_fen(), board.hash(), board.checkers());
        child.make_move(mov);
        assert_ne!(
            child.hash(),
            board.hash(),
            "make_move must change the hash: {mov:?}"
        );
        child.undo_move(mov);
        assert_eq!(
            (child.to_fen(), child.hash(), child.checkers()),
            before,
            "undo_move must fully restore the board: {mov:?}"
        );
        tested += 1;
    }
    assert!(tested > 0, "no legal moves found for FEN: {fen}");
}

#[test]
fn test_make_undo_start_pos() {
    test_make_undo_for_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
}

#[test]
fn test_make_undo_kiwipete() {
    test_make_undo_for_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
}

#[test]
fn test_make_undo_pos3() {
    test_make_undo_for_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
}

#[test]
fn test_make_undo_pos4() {
    test_make_undo_for_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
}

#[test]
fn test_make_undo_pos5() {
    test_make_undo_for_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
}

#[test]
fn test_make_undo_pos6() {
    test_make_undo_for_fen(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    );
}

#[test]
fn test_make_undo_en_passant() {
    test_make_undo_for_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
}

#[test]
fn test_make_undo_promotions() {
    test_make_undo_for_fen("r1bqkb1r/pPpppppp/2n2n2/8/8/8/P1PPPPPP/RNBQKBNR w KQkq - 0 5");
}

#[test]
fn test_make_undo_castling() {
    test_make_undo_for_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
}
