use crate::common::{board, find_move};
use chess_core::prelude::*;
use chess_engine::move_gen::scoring::see_ge;

#[test]
fn test_see() {
    // (position, move, exact exchange value)
    let cases = [
        // Undefended queen
        ("4k3/8/8/3q4/8/2N5/8/4K3 w - - 0 1", "c3d5", 900),
        // Bishop for knight
        (
            "rnbqkb1r/pppppppp/5n2/4B3/8/8/PPPPPPPP/RN1QKBNR w KQkq - 0 1",
            "e5f6",
            0,
        ),
        // Queen for pawn
        (
            "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5Q2/PPPP1PPP/RNB1KBNR w KQkq - 0 1",
            "f3f7",
            -800,
        ),
        (
            "1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1",
            "e1e5",
            100,
        ),
        (
            "1k2r3/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1",
            "e1e5",
            -400,
        ),
        // X-ray batteries on both sides
        (
            "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1",
            "d3e5",
            -200,
        ),
        ("4k3/8/8/3Pp3/8/8/8/4K3 w - e6 0 1", "d5e6", 100),
        // Promotions gain the queen minus the pawn
        ("5r2/4P3/8/8/8/8/8/4K2k w - - 0 1", "e7f8q", 1300),
        ("8/4P1k1/8/8/8/8/8/4K3 w - - 0 1", "e7e8q", 800),
        // Quiet moves onto attacked squares
        ("4k3/8/4p3/8/8/2N5/8/4K3 w - - 0 1", "c3d5", -300),
        ("4k3/8/4p3/8/8/2N5/8/4K3 w - - 0 1", "c3b5", 0),
    ];
    for (fen, uci, value) in cases {
        let board = board(fen);
        let mov = find_move(&board, uci);
        assert!(see_ge(mov, &board, value), "{uci} in {fen} below {value}");
        assert!(
            !see_ge(mov, &board, value + 1),
            "{uci} in {fen} above {value}"
        );
    }

    // The king can't capture a defended piece
    let board = board("4r1k1/8/8/8/8/4r3/4K3/8 w - - 0 1");
    let mov = Move::new(Sq::E2, Sq::E3, MoveFlags::Capture);
    assert!(!see_ge(mov, &board, 0));
}
