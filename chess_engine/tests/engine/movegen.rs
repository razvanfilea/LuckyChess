use crate::common::{PERFT_SUITE, board, legal_moves};
use chess_core::prelude::*;
use chess_engine::perft::perft;
use chess_engine::uci::format_move;

#[test]
#[cfg_attr(miri, ignore)]
fn test_perft_suite() {
    for &(fen, expected) in PERFT_SUITE {
        let depth = expected.len() as u8;
        let nodes = perft(&mut board(fen), depth);
        assert_eq!(
            nodes,
            expected[expected.len() - 1],
            "perft({depth}) of {fen}"
        );
    }
}

#[test]
fn test_legal_moves() {
    // (position, some legal moves, some illegal moves)
    let cases: &[(&str, &[&str], &[&str])] = &[
        // Pinned pieces only move along the pin
        (
            "4r2k/8/8/8/8/4R3/8/4K3 w - - 0 1",
            &["e3e8", "e3e4", "e3e2"],
            &["e3d3", "e3f3"],
        ),
        (
            "7k/8/8/b7/8/8/3B4/4K3 w - - 0 1",
            &["d2c3", "d2a5"],
            &["d2e3", "d2c1"],
        ),
        (
            "4r2k/8/8/8/8/8/4N3/4K3 w - - 0 1",
            &["e1d1"],
            &["e2c1", "e2c3", "e2d4", "e2f4", "e2g3", "e2g1"],
        ),
        ("4r2k/8/8/8/8/8/4P3/4K3 w - - 0 1", &["e2e3", "e2e4"], &[]),
        (
            "7k/8/8/8/8/2b5/3P4/4K3 w - - 0 1",
            &["d2c3"],
            &["d2d3", "d2d4"],
        ),
        (
            "4q2k/8/8/8/8/8/4B3/4K3 w - - 0 1",
            &["e1d1"],
            &["e2d3", "e2f3"],
        ),
        (
            "7k/8/8/q7/8/8/3R4/4K3 w - - 0 1",
            &["e1e2"],
            &["d2d3", "d2c2"],
        ),
        // The king can't step into check or capture a defended piece
        (
            "3r3k/8/8/8/8/8/8/4K3 w - - 0 1",
            &["e1e2", "e1f1"],
            &["e1d1", "e1d2"],
        ),
        (
            "7k/8/8/8/8/3p4/4p3/4K3 w - - 0 1",
            &["e1d2"],
            &["e1e2", "e1d1", "e1f1"],
        ),
        ("7k/8/8/8/8/8/4p3/4K3 w - - 0 1", &["e1e2"], &[]),
        (
            "4k3/8/8/8/8/8/8/3R2K1 b - - 0 1",
            &["e8e7", "e8f8"],
            &["e8d8", "e8d7"],
        ),
        // Castling
        (
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
            &["e1g1", "e1c1"],
            &[],
        ),
        ("r3k2r/8/8/8/8/8/8/R3K2R w K - 0 1", &["e1g1"], &["e1c1"]),
        (
            "r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1",
            &["e1g1"],
            &["e1c1"],
        ),
        (
            "r3k2r/8/8/8/8/8/8/R2NKB1R w KQkq - 0 1",
            &[],
            &["e1c1", "e1g1"],
        ),
        ("4r2k/8/8/8/8/8/8/R3K2R w KQ - 0 1", &[], &["e1g1", "e1c1"]),
        ("5r1k/8/8/8/8/8/8/R3K2R w KQ - 0 1", &["e1c1"], &["e1g1"]),
        ("6rk/8/8/8/8/8/8/R3K2R w KQ - 0 1", &["e1c1"], &["e1g1"]),
        ("3r3k/8/8/8/8/8/8/R3K2R w KQ - 0 1", &["e1g1"], &["e1c1"]),
        // Only the king's path matters, not b1
        ("1r5k/8/8/8/8/8/8/R3K2R w KQ - 0 1", &["e1g1", "e1c1"], &[]),
        ("r3k2r/8/8/8/8/8/8/5RK1 b kq - 0 1", &["e8c8"], &["e8g8"]),
        ("r3k2r/8/8/8/8/8/8/4R1K1 b kq - 0 1", &[], &["e8g8", "e8c8"]),
        // En passant
        ("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", &["e5d6"], &[]),
        ("4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1", &["e4d3"], &[]),
        ("8/8/8/KPp4r/8/8/8/7k w - c6 0 1", &["b5b6"], &["b5c6"]),
        ("4k3/8/8/3pP3/2K5/8/8/8 w - d6 0 1", &["e5d6"], &["e5e6"]),
        // Evasions: block, capture the checker, or move the king
        (
            "4r2k/8/8/8/8/2N5/8/4K3 w - - 0 1",
            &["c3e2", "c3e4"],
            &["c3d5", "c3b5"],
        ),
        (
            "7k/8/8/8/8/3n4/2P1P3/4K3 w - - 0 1",
            &["c2d3", "e2d3"],
            &["c2c3", "e2e3", "e2e4"],
        ),
        // Double check: only the king moves, and not along the checking lines
        (
            "b3r2k/8/8/8/4K3/8/3N4/8 w - - 0 1",
            &["e4f4"],
            &["e4f3", "e4e3", "d2c4", "d2f3", "d2b3"],
        ),
        // Promotions to every piece
        (
            "3q3k/4P3/8/8/8/8/8/K7 w - - 0 1",
            &["e7e8q", "e7e8r", "e7e8b", "e7e8n", "e7d8q", "e7d8n"],
            &["e7e8"],
        ),
    ];

    for &(fen, legal, illegal) in cases {
        let moves: Vec<String> = legal_moves(&board(fen))
            .into_iter()
            .map(format_move)
            .collect();
        for &uci in legal {
            assert!(
                moves.iter().any(|m| m == uci),
                "{uci} should be legal in {fen}"
            );
        }
        for &uci in illegal {
            assert!(Sq::parse(&uci[2..]).is_some(), "malformed move {uci}");
            assert!(
                !moves.iter().any(|m| m == uci),
                "{uci} should be illegal in {fen}"
            );
        }
    }
}
