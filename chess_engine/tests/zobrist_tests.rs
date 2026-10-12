use chess_core::prelude::*;
use chess_engine::board::Board;

#[test]
fn test_hash_determinism() {
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    ];
    for fen in fens {
        let a = Board::from_fen(fen).unwrap();
        let b = Board::from_fen(fen).unwrap();
        assert_eq!(a.hash(), b.hash(), "non-deterministic hash for: {fen}");
    }
}

#[test]
fn test_hash_discrimination_side() {
    let w = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let b = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    assert_ne!(w.hash(), b.hash());
}

#[test]
fn test_hash_discrimination_castling() {
    let full = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let none = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1").unwrap();
    assert_ne!(full.hash(), none.hash());
}

#[test]
fn test_hash_discrimination_ep() {
    let with_ep =
        Board::from_fen("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    let no_ep =
        Board::from_fen("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
    assert_ne!(with_ep.hash(), no_ep.hash());
}

#[test]
fn test_hash_incremental_equals_from_scratch() {
    let mut board = Board::start_pos();
    board.make_move(Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn));
    let expected =
        Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    assert_eq!(board.hash(), expected.hash());
}

#[test]
fn test_hash_incremental_ruy_lopez() {
    let mut board = Board::start_pos();
    let moves = [
        Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn),
        Move::new(Sq::E7, Sq::E5, MoveFlags::DoublePawn),
        Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet),
        Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet),
        Move::new(Sq::F1, Sq::B5, MoveFlags::Quiet),
    ];
    for m in moves {
        board.make_move(m);
    }
    let expected =
        Board::from_fen("r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3")
            .unwrap();
    assert_eq!(board.hash(), expected.hash());
}

#[test]
fn test_hash_transposition() {
    // Order A: g1f3, b8c6, b1c3, g8f6
    let mut board_a = Board::start_pos();
    board_a.make_move(Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet));
    board_a.make_move(Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet));
    board_a.make_move(Move::new(Sq::B1, Sq::C3, MoveFlags::Quiet));
    board_a.make_move(Move::new(Sq::G8, Sq::F6, MoveFlags::Quiet));

    // Order B: b1c3, g8f6, g1f3, b8c6
    let mut board_b = Board::start_pos();
    board_b.make_move(Move::new(Sq::B1, Sq::C3, MoveFlags::Quiet));
    board_b.make_move(Move::new(Sq::G8, Sq::F6, MoveFlags::Quiet));
    board_b.make_move(Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet));
    board_b.make_move(Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet));

    assert_eq!(board_a.hash(), board_b.hash());
}

#[test]
fn test_hash_undo_restores() {
    let mut board = Board::start_pos();
    let h = board.hash();
    let mov = Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn);
    board.make_move(mov);
    assert_ne!(board.hash(), h);
    board.undo_move(mov);
    assert_eq!(board.hash(), h);
}
