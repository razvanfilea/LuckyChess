use chess_core::prelude::*;
use chess_engine::board::Board;

#[test]
fn test_start_pos() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = Board::from_fen(fen).expect("Startpos FEN should be valid");

    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 0);

    // Verify pieces on 1st rank
    assert_eq!(
        board.piece_at(Sq::A1),
        Some(ColoredPiece::new(Piece::Rook, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::B1),
        Some(ColoredPiece::new(Piece::Knight, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::C1),
        Some(ColoredPiece::new(Piece::Bishop, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::D1),
        Some(ColoredPiece::new(Piece::Queen, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::E1),
        Some(ColoredPiece::new(Piece::King, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::F1),
        Some(ColoredPiece::new(Piece::Bishop, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::G1),
        Some(ColoredPiece::new(Piece::Knight, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::H1),
        Some(ColoredPiece::new(Piece::Rook, Color::White))
    );

    // Verify pieces on 8th rank
    assert_eq!(
        board.piece_at(Sq::A8),
        Some(ColoredPiece::new(Piece::Rook, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::B8),
        Some(ColoredPiece::new(Piece::Knight, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::C8),
        Some(ColoredPiece::new(Piece::Bishop, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::D8),
        Some(ColoredPiece::new(Piece::Queen, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::E8),
        Some(ColoredPiece::new(Piece::King, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::F8),
        Some(ColoredPiece::new(Piece::Bishop, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::G8),
        Some(ColoredPiece::new(Piece::Knight, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::H8),
        Some(ColoredPiece::new(Piece::Rook, Color::Black))
    );

    // Verify pawns
    for file in 0..8 {
        let white_pawn = Sq::new(file, 1).unwrap();
        let black_pawn = Sq::new(file, 6).unwrap();
        assert_eq!(
            board.piece_at(white_pawn),
            Some(ColoredPiece::new(Piece::Pawn, Color::White))
        );
        assert_eq!(
            board.piece_at(black_pawn),
            Some(ColoredPiece::new(Piece::Pawn, Color::Black))
        );
    }

    // Verify empty squares in middle
    for rank in 2..=5 {
        for file in 0..8 {
            let empty_sq = Sq::new(file, rank).unwrap();
            assert_eq!(board.piece_at(empty_sq), None);
        }
    }
}

#[test]
fn test_missing_trailing_fields() {
    // 1. Piece placement only (EPD)
    let fen_epd = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
    let board = Board::from_fen(fen_epd).expect("Piece placement only should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::empty());
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 0);
    assert_eq!(board.piece_at(Sq::E1).unwrap().piece(), Piece::King);

    // 2. Piece placement + side to move (Black)
    let fen_side = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b";
    let board = Board::from_fen(fen_side).expect("Piece placement + side should parse");
    assert_eq!(board.to_play, Color::Black);
    assert_eq!(board.castling_rights(), CastlingRights::empty());
    assert_eq!(board.ply, 1);

    // 3. Piece placement + side + castling
    let fen_castling = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq";
    let board =
        Board::from_fen(fen_castling).expect("Piece placement + side + castling should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_eq!(board.ply, 0);

    // 4. 4 tokens (no halfmove / fullmove)
    let fen_4_tokens = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -";
    let board = Board::from_fen(fen_4_tokens).expect("4 tokens should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 0);
}

#[test]
fn test_castling_rights_variations() {
    // White kingside only
    let fen = "8/8/8/8/8/8/8/4K2R w K - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.castling_rights(), CastlingRights::WHITE_00);

    // Black queenside only
    let fen = "r3k3/8/8/8/8/8/8/8 b q - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.castling_rights(), CastlingRights::BLACK_000);

    // Mixed: White Queenside + Black Kingside
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w Qk - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(
        board.castling_rights(),
        CastlingRights::WHITE_000 | CastlingRights::BLACK_00
    );

    // Explicit dash '-'
    let fen = "8/8/8/8/8/8/8/4K3 w - - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.castling_rights(), CastlingRights::empty());
}

#[test]
fn test_en_passant_squares() {
    // Valid: White to move, Black moved e7-e5 -> target is e6 (White has pawn on d5 able to capture)
    let fen_white = "rnbqkbnr/pppp1ppp/8/3Pp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 2";
    let board_white = Board::from_fen(fen_white).unwrap();
    assert_eq!(board_white.en_passant_target_sq(), Some(Sq::E6));

    // Valid: Black to move, White moved e2-e4 -> target is e3 (Black has pawn on d4 able to capture)
    let fen_black = "rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPP2PPP/RNBQKBNR b KQkq e3 0 1";
    let board_black = Board::from_fen(fen_black).unwrap();
    assert_eq!(board_black.en_passant_target_sq(), Some(Sq::E3));

    // Invalid / Discarded: White to move, target e6 but White has no pawn able to capture
    let fen_no_capturer = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2";
    let board_no_capturer = Board::from_fen(fen_no_capturer).unwrap();
    assert_eq!(board_no_capturer.en_passant_target_sq(), None);

    // Invalid: White to move but EP square is rank 3 (illegal rank for White to move)
    let fen_invalid = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e3 0 1";
    let board_invalid = Board::from_fen(fen_invalid).unwrap();
    assert_eq!(board_invalid.en_passant_target_sq(), None);

    // Explicit '-'
    let fen_none = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board_none = Board::from_fen(fen_none).unwrap();
    assert_eq!(board_none.en_passant_target_sq(), None);
}

#[test]
fn test_move_counters_and_ply() {
    // Move 1 White -> ply 0
    let b1 = Board::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap();
    assert_eq!(b1.half_move_clock(), 0);
    assert_eq!(b1.ply, 0);

    // Move 1 Black -> ply 1
    let b2 = Board::from_fen("8/8/8/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(b2.ply, 1);

    // Move 20 White -> ply 38
    let b3 = Board::from_fen("8/8/8/8/8/8/8/8 w - - 12 20").unwrap();
    assert_eq!(b3.half_move_clock(), 12);
    assert_eq!(b3.ply, 38);

    // Move 20 Black -> ply 39
    let b4 = Board::from_fen("8/8/8/8/8/8/8/8 b - - 12 20").unwrap();
    assert_eq!(b4.ply, 39);

    // High half-move clock (e.g. 75 plies)
    let b5 = Board::from_fen("8/8/8/8/8/8/8/8 w - - 75 50").unwrap();
    assert_eq!(b5.half_move_clock(), 75);
    assert_eq!(b5.ply, 98);
}

#[test]
fn test_standard_chess_positions() {
    // Kiwipete
    let kiwipete = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let board = Board::from_fen(kiwipete).expect("Kiwipete should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_eq!(
        board.piece_at(Sq::E5),
        Some(ColoredPiece::new(Piece::Knight, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::F3),
        Some(ColoredPiece::new(Piece::Queen, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::E7),
        Some(ColoredPiece::new(Piece::Queen, Color::Black))
    );

    // Endgame Position (Pos 3)
    let pos3 = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    let board = Board::from_fen(pos3).expect("Position 3 should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::empty());
    assert_eq!(
        board.piece_at(Sq::A5),
        Some(ColoredPiece::new(Piece::King, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::H4),
        Some(ColoredPiece::new(Piece::King, Color::Black))
    );

    // Empty board
    let empty = "8/8/8/8/8/8/8/8 w - - 0 1";
    let board = Board::from_fen(empty).unwrap();
    for sq in 0..64 {
        assert_eq!(board.piece_at(Sq::from_raw(sq as u8).unwrap()), None);
    }
}

#[test]
fn test_resilience_and_malformed_fen() {
    // Irregular whitespace
    let messy_spaces = "  rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR    w   KQkq   -   0   1  ";
    let board = Board::from_fen(messy_spaces).expect("Messy whitespace should parse");
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);

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
fn test_half_move_clock_saturates() {
    let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 300 200").unwrap();
    assert_eq!(board.half_move_clock(), u8::MAX);
}

#[test]
fn test_castling_rights_need_king_and_rook() {
    let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w KQkq - 0 1").unwrap();
    assert_eq!(board.castling_rights(), CastlingRights::empty());

    let board = Board::from_fen("r3k3/8/8/8/8/8/8/4K2R w KQkq - 0 1").unwrap();
    assert_eq!(
        board.castling_rights(),
        CastlingRights::WHITE_00 | CastlingRights::BLACK_000
    );

    let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R2K3R w KQkq - 0 1").unwrap();
    assert_eq!(
        board.castling_rights(),
        CastlingRights::BLACK_00 | CastlingRights::BLACK_000
    );
}

#[test]
fn test_bitboards_accuracy_startpos() {
    let board = Board::start_pos();

    // White occupies ranks 1 and 2 (bits 0..16) -> 0x0000_0000_0000_FFFF
    assert_eq!(board.colors(Color::White), 0x0000_0000_0000_FFFF);

    // Black occupies ranks 7 and 8 (bits 48..64) -> 0xFFFF_0000_0000_0000
    assert_eq!(board.colors(Color::Black), 0xFFFF_0000_0000_0000);

    // Total occupied
    assert_eq!(board.occupied(), 0xFFFF_0000_0000_FFFF);

    // Pawns: ranks 2 and 7 -> 0x00FF_0000_0000_FF00
    assert_eq!(board.pieces(Piece::Pawn), 0x00FF_0000_0000_FF00);

    // Kings: E1 (bit 4) and E8 (bit 60)
    let expected_kings = Sq::E1.bitboard() | Sq::E8.bitboard();
    assert_eq!(board.pieces(Piece::King), expected_kings);

    // Queens: D1 (bit 3) and D8 (bit 59)
    let expected_queens = Sq::D1.bitboard() | Sq::D8.bitboard();
    assert_eq!(board.pieces(Piece::Queen), expected_queens);

    // Rooks: A1, H1, A8, H8
    let expected_rooks =
        Sq::A1.bitboard() | Sq::H1.bitboard() | Sq::A8.bitboard() | Sq::H8.bitboard();
    assert_eq!(board.pieces(Piece::Rook), expected_rooks);

    // Knights: B1, G1, B8, G8
    let expected_knights =
        Sq::B1.bitboard() | Sq::G1.bitboard() | Sq::B8.bitboard() | Sq::G8.bitboard();
    assert_eq!(board.pieces(Piece::Knight), expected_knights);

    // Bishops: C1, F1, C8, F8
    let expected_bishops =
        Sq::C1.bitboard() | Sq::F1.bitboard() | Sq::C8.bitboard() | Sq::F8.bitboard();
    assert_eq!(board.pieces(Piece::Bishop), expected_bishops);
}

#[test]
fn test_perft_suite_positions() {
    // Position 4 (Talkchess) - Promotion and complex pawn structure
    let pos4 = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    let board4 = Board::from_fen(pos4).expect("Pos 4 should parse");
    assert_eq!(board4.to_play, Color::White);
    assert_eq!(
        board4.castling_rights(),
        CastlingRights::BLACK_00 | CastlingRights::BLACK_000
    );
    assert_eq!(
        board4.piece_at(Sq::A7),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_eq!(
        board4.piece_at(Sq::B2),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );

    // Position 5 - Check and heavy tactical piece placement
    let pos5 = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    let board5 = Board::from_fen(pos5).expect("Pos 5 should parse");
    assert_eq!(board5.to_play, Color::White);
    assert_eq!(
        board5.castling_rights(),
        CastlingRights::WHITE_00 | CastlingRights::WHITE_000
    );
    assert_eq!(board5.half_move_clock(), 1);
    assert_eq!(board5.ply, 14); // Move 8, White to play -> (8 - 1) * 2 = 14

    // Position 6 - Symmetric middlegame
    let pos6 = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
    let board6 = Board::from_fen(pos6).expect("Pos 6 should parse");
    assert_eq!(board6.to_play, Color::White);
    assert_eq!(board6.castling_rights(), CastlingRights::empty());
    assert_eq!(board6.ply, 18); // Move 10, White to play -> (10 - 1) * 2 = 18
}

fn rank_with_pawn(file: usize, ch: char) -> String {
    let mut s = String::new();
    if file > 0 {
        s.push_str(&file.to_string());
    }
    s.push(ch);
    if file < 7 {
        s.push_str(&(7 - file).to_string());
    }
    s
}

#[test]
fn test_all_en_passant_files() {
    // Test all files a-h for Black double-push (target rank 6 for White)
    let files = ["a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6"];
    for (i, &f) in files.iter().enumerate() {
        let adj_file = if i > 0 { i - 1 } else { i + 1 };
        let fen = format!(
            "8/8/8/{}/8/8/8/8 w - {f} 0 1",
            rank_with_pawn(adj_file, 'P')
        );
        let board = Board::from_fen(&fen).unwrap();
        let expected_sq = Sq::new(i as u8, 5); // rank 6 is index 5
        assert_eq!(board.en_passant_target_sq(), expected_sq);

        // Without attacker, EP square is ignored
        let empty_fen = format!("8/8/8/8/8/8/8/8 w - {f} 0 1");
        assert_eq!(
            Board::from_fen(&empty_fen).unwrap().en_passant_target_sq(),
            None
        );
    }

    // Test all files a-h for White double-push (target rank 3 for Black)
    let files_black = ["a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3"];
    for (i, &f) in files_black.iter().enumerate() {
        let adj_file = if i > 0 { i - 1 } else { i + 1 };
        let fen = format!(
            "8/8/8/8/{}/8/8/8 b - {f} 0 1",
            rank_with_pawn(adj_file, 'p')
        );
        let board = Board::from_fen(&fen).unwrap();
        let expected_sq = Sq::new(i as u8, 2); // rank 3 is index 2
        assert_eq!(board.en_passant_target_sq(), expected_sq);

        // Without attacker, EP square is ignored
        let empty_fen = format!("8/8/8/8/8/8/8/8 b - {f} 0 1");
        assert_eq!(
            Board::from_fen(&empty_fen).unwrap().en_passant_target_sq(),
            None
        );
    }
}

#[test]
fn test_multiple_promoted_pieces() {
    // 9 White Queens on board
    let fen_queens = "QQQQkQQQ/8/8/8/8/8/8/4K1Q1 w - - 0 1";
    let board = Board::from_fen(fen_queens).expect("Board with multiple queens should parse");
    assert_eq!(
        board.piece_at(Sq::E8),
        Some(ColoredPiece::new(Piece::King, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::E1),
        Some(ColoredPiece::new(Piece::King, Color::White))
    );

    // Count queens in bitboard
    let queen_bb = board.pieces(Piece::Queen);
    assert_eq!(queen_bb.count_ones(), 8);
}
