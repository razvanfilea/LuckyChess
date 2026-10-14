use crate::common::{PERFT_SUITE, START_FEN, board, play};
use chess_core::prelude::*;
use chess_engine::board::Board;

#[test]
fn test_fen_round_trip() {
    let fens = PERFT_SUITE.iter().map(|&(fen, _)| fen).chain([
        "rnbqkbnr/pppp1ppp/8/3Pp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 2",
        "rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPP2PPP/RNBQKBNR b KQkq e3 0 1",
        "r3k2r/8/8/8/8/8/8/R3K2R w Qk - 0 1",
        "QQQQkQQQ/8/8/8/8/8/8/4K1Q1 w - - 0 1",
        "8/8/8/8/8/8/8/8 b - - 75 50",
    ]);
    for fen in fens {
        assert_eq!(board(fen).to_fen(), fen);
    }
}

#[test]
fn test_fen_normalization() {
    let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
    let cases = [
        // Missing trailing fields
        (
            start,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1",
        ),
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b - - 0 1",
        ),
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq",
            START_FEN,
        ),
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -",
            START_FEN,
        ),
        (
            "  rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR    w   KQkq   -   0   1  ",
            START_FEN,
        ),
        // En passant square is dropped when no pawn can capture, or on the wrong rank
        (
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
        ),
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e3 0 1",
            START_FEN,
        ),
        // Castling rights need the king and rook on their home squares
        (
            "4k3/8/8/8/8/8/8/4K3 w KQkq - 0 1",
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
        ),
        (
            "r3k3/8/8/8/8/8/8/4K2R w KQkq - 0 1",
            "r3k3/8/8/8/8/8/8/4K2R w Kq - 0 1",
        ),
        (
            "r3k2r/8/8/8/8/8/8/R2K3R w KQkq - 0 1",
            "r3k2r/8/8/8/8/8/8/R2K3R w kq - 0 1",
        ),
        // The half-move clock saturates
        (
            "4k3/8/8/8/8/8/8/4K3 w - - 300 200",
            "4k3/8/8/8/8/8/8/4K3 w - - 255 200",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(board(input).to_fen(), expected, "{input:?}");
    }
}

#[test]
fn test_fen_fields() {
    let start = Board::start_pos();
    assert_eq!(start.to_fen(), START_FEN);
    assert_eq!(start.colors(Color::White), 0x0000_0000_0000_FFFF);
    assert_eq!(start.colors(Color::Black), 0xFFFF_0000_0000_0000);
    assert_eq!(start.pieces(Piece::Pawn), 0x00FF_0000_0000_FF00);
    assert_eq!(start.pieces(Piece::Rook), 0x8100_0000_0000_0081);
    assert_eq!(start.pieces(Piece::Knight), 0x4200_0000_0000_0042);
    assert_eq!(start.pieces(Piece::Bishop), 0x2400_0000_0000_0024);
    assert_eq!(start.pieces(Piece::Queen), 0x0800_0000_0000_0008);
    assert_eq!(start.pieces(Piece::King), 0x1000_0000_0000_0010);
    assert_eq!(
        start.piece_at(Sq::D8),
        Some(ColoredPiece::new(Piece::Queen, Color::Black))
    );
    assert_eq!(start.piece_at(Sq::E4), None);

    let pos = board("4k3/8/8/3pP3/8/8/8/4K3 w - d6 12 20");
    assert_eq!(pos.to_play, Color::White);
    assert_eq!(pos.en_passant_target_sq(), Some(Sq::D6));
    assert_eq!(pos.castling_rights(), CastlingRights::empty());
    assert_eq!(pos.half_move_clock(), 12);
    assert_eq!(pos.ply, 38);
    assert_eq!(board("8/8/8/8/8/8/8/8 b - - 0 1").ply, 1);
}

#[test]
fn test_malformed_fen() {
    for bad in [
        "",
        "     ",
        "not a fen",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR!#$ w KQkq - 0 1",
        "pppppppppppp/8/8/8/8/8/8/8 w - - 0 1",
        "ppppppp/8/8/8/8/8/8/8 w - - 0 1",
        "44/8/8/8/8/8/8/8 w - - 0 1",
        "9/8/8/8/8/8/8/8 w - - 0 1",
        "0p7/8/8/8/8/8/8/8 w - - 0 1",
        "8/8/8/8/8/8/8 w - - 0 1",
        "8/8/8/8/8/8/8/8/8/8/8 w - - 0 1",
        "8/8/8/8/8/8/8/8 x - - 0 1",
        "8/8/8/8/8/8/8/8 W - - 0 1",
        "8/8/8/8/8/8/8/8 w KX - 0 1",
        "8/8/8/8/8/8/8/8 w - e3x 0 1",
        "8/8/8/8/8/8/8/8 w - zz 0 1",
        "8/8/8/8/8/8/8/8 w - - abc 1",
        "8/8/8/8/8/8/8/8 w - - 0 -1",
        "8/8/8/8/8/8/8/8 w - - 0 1 extra",
    ] {
        assert!(Board::from_fen(bad).is_none(), "accepted: {bad:?}");
    }
}

#[test]
fn test_make_move() {
    let castling = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
    let castling_black = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
    let rooks = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let rooks_black = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1";
    // (before, moves, after)
    let cases = [
        // Quiet moves bump the half-move clock, pawn moves reset it
        (
            START_FEN,
            "g1f3",
            "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
        ),
        (
            START_FEN,
            "e2e3",
            "rnbqkbnr/pppppppp/8/8/8/4P3/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
        ),
        // A double push sets the en passant square only if it can be captured
        (
            START_FEN,
            "e2e4",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
        ),
        (
            "rnbqkbnr/pppppppp/8/8/3p4/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "e2e4",
            "rnbqkbnr/pppppppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        ),
        (
            "rnbqkbnr/pppppppp/8/8/3p4/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "e2e4 g8f6",
            "rnbqkb1r/pppppppp/5n2/8/3pP3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
        ),
        // En passant removes the pawn behind the target square
        (
            "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
            "e5d6",
            "rnbqkbnr/ppp1pppp/3P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2",
        ),
        (
            "rnbqkbnr/pppppp1p/8/8/6Pp/8/PPPPPP1P/RNBQKBNR b KQkq g3 0 2",
            "h4g3",
            "rnbqkbnr/pppppp1p/8/8/8/6p1/PPPPPP1P/RNBQKBNR w KQkq - 0 3",
        ),
        (
            "rnbqkbnr/1ppppppp/8/P7/8/8/1PPPPPPP/RNBQKBNR b KQkq - 0 2",
            "b7b5 a5b6",
            "rnbqkbnr/2pppppp/1P6/8/8/8/1PPPPPPP/RNBQKBNR b KQkq - 0 3",
        ),
        // Captures reset the half-move clock
        (
            "r1bqk2r/pppp1ppp/2n5/4p3/1bB1n3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 5",
            "c3e4",
            "r1bqk2r/pppp1ppp/2n5/4p3/1bB1N3/5N2/PPPP1PPP/R1BQK2R b KQkq - 0 5",
        ),
        // Castling
        (
            castling,
            "e1g1",
            "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 b kq - 1 1",
        ),
        (
            castling,
            "e1c1",
            "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R b kq - 1 1",
        ),
        (
            castling_black,
            "e8g8",
            "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQ - 1 2",
        ),
        (
            castling_black,
            "e8c8",
            "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQ - 1 2",
        ),
        // Castling rights are lost by moving or capturing the king or rooks
        (rooks, "e1e2", "r3k2r/8/8/8/8/8/4K3/R6R b kq - 1 1"),
        (rooks, "a1b1", "r3k2r/8/8/8/8/8/8/1R2K2R b Kkq - 1 1"),
        (rooks, "a1a8", "R3k2r/8/8/8/8/8/8/4K2R b Kk - 0 1"),
        (rooks_black, "h8g8", "r3k1r1/8/8/8/8/8/8/R3K2R w KQq - 1 2"),
        (rooks_black, "h8h1", "r3k3/8/8/8/8/8/8/R3K2r w Qq - 0 2"),
        // Promotions
        (
            "8/4P3/8/8/8/8/8/4K2k w - - 0 1",
            "e7e8q",
            "4Q3/8/8/8/8/8/8/4K2k b - - 0 1",
        ),
        (
            "8/4P3/8/8/8/8/8/4K2k w - - 0 1",
            "e7e8n",
            "4N3/8/8/8/8/8/8/4K2k b - - 0 1",
        ),
        (
            "4K2k/8/8/8/8/8/4p3/8 b - - 0 1",
            "e2e1r",
            "4K2k/8/8/8/8/8/8/4r3 w - - 0 2",
        ),
        (
            "4K2k/8/8/8/8/8/4p3/8 b - - 0 1",
            "e2e1b",
            "4K2k/8/8/8/8/8/8/4b3 w - - 0 2",
        ),
        (
            "3r3k/4P3/8/8/8/8/8/4K3 w - - 0 1",
            "e7d8q",
            "3Q3k/8/8/8/8/8/8/4K3 b - - 0 1",
        ),
        (
            "4K3/8/8/8/8/8/3p4/2B3k1 b - - 0 1",
            "d2c1n",
            "4K3/8/8/8/8/8/8/2n3k1 w - - 0 2",
        ),
        (
            "r3k2r/1P6/8/8/8/8/8/4K2R w Kkq - 0 1",
            "b7a8q",
            "Q3k2r/8/8/8/8/8/8/4K2R b Kk - 0 1",
        ),
        // Games
        (
            START_FEN,
            "f2f3 e7e5 g2g4 d8h4",
            "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
        ),
        (
            START_FEN,
            "e2e4 e7e5 d1h5 b8c6 f1c4 g8f6 h5f7",
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        ),
        (
            START_FEN,
            "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f8e7 f1e1 b7b5 a4b3 d7d6 c2c3 e8g8",
            "r1bq1rk1/2p1bppp/p1np1n2/1p2p3/4P3/1BP2N2/PP1P1PPP/RNBQR1K1 w - - 1 9",
        ),
    ];
    for (before, moves, after) in cases {
        let mut pos = board(before);
        play(&mut pos, moves);
        assert_eq!(pos.to_fen(), after, "{before} + {moves}");
        assert_eq!(pos.hash(), board(after).hash());
    }
}

#[test]
fn test_draws() {
    let cases = [
        (START_FEN, false),
        // Insufficient material
        ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", true),
        ("4k3/8/8/8/8/8/4N3/4K3 w - - 0 1", true),
        ("4k3/8/8/8/8/8/4B3/4K3 w - - 0 1", true),
        ("6bk/8/8/8/8/8/8/KB6 w - - 0 1", true),
        ("5b1k/8/8/8/8/8/8/KB6 w - - 0 1", false),
        ("7k/8/8/8/8/8/8/KNN5 w - - 0 1", false),
        ("7k/8/8/8/8/8/8/KR6 w - - 0 1", false),
        ("4k3/8/8/8/8/8/P7/K7 w - - 0 1", false),
        // Fifty-move rule
        ("4k3/8/8/8/8/8/4P3/4K3 w - - 100 1", true),
        ("4k3/8/8/8/8/8/4P3/4K3 w - - 99 1", false),
    ];
    for (fen, draw) in cases {
        assert_eq!(board(fen).is_draw(), draw, "{fen}");
    }
}

#[test]
fn test_hash_covers_state() {
    let hash = |fen| board(fen).hash();
    let start = hash(START_FEN);
    assert_ne!(
        start,
        hash("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1")
    );
    assert_ne!(
        start,
        hash("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Kkq - 0 1")
    );
    assert_ne!(
        hash("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"),
        hash("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
    );
    // Move counters are not part of the hash
    assert_eq!(
        start,
        hash("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 7 30")
    );
}
