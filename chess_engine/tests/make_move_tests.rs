use chess_core::prelude::*;
use chess_engine::board::Board;

/// Helper function to verify internal board consistency and invariants after any move.
fn assert_board_invariants(board: &Board) {
    let white_bb = board.colors(Color::White);
    let black_bb = board.colors(Color::Black);

    // 1. Color bitboards must be disjoint
    assert_eq!(
        white_bb & black_bb,
        0,
        "White and Black color bitboards overlap!"
    );

    // 2. Total occupancy must equal White | Black
    assert_eq!(
        board.occupied(),
        white_bb | black_bb,
        "Occupied bitboard does not equal white_bb | black_bb"
    );
    assert_eq!(
        board.empty(),
        !board.occupied(),
        "Empty bitboard is not !occupied"
    );

    // 3. Piece bitboards must be pairwise disjoint and their union must equal occupied
    let pawns = board.pieces(Piece::Pawn);
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);
    let kings = board.pieces(Piece::King);

    let all_pieces = [
        (Piece::Pawn, pawns),
        (Piece::Knight, knights),
        (Piece::Bishop, bishops),
        (Piece::Rook, rooks),
        (Piece::Queen, queens),
        (Piece::King, kings),
    ];

    for i in 0..all_pieces.len() {
        for j in (i + 1)..all_pieces.len() {
            assert_eq!(
                all_pieces[i].1 & all_pieces[j].1,
                0,
                "Piece bitboards {:?} and {:?} overlap!",
                all_pieces[i].0,
                all_pieces[j].0
            );
        }
    }

    let union_pieces = pawns | knights | bishops | rooks | queens | kings;
    assert_eq!(
        union_pieces,
        board.occupied(),
        "Union of piece bitboards does not match occupied bitboard"
    );

    // 4. Mailbox must exactly match the bitboards
    for sq_idx in 0..Sq::NB {
        let sq = unsafe { Sq::from_raw_unchecked(sq_idx as u8) };
        let sq_bb = sq.bitboard();
        let piece_opt = board.piece_at(sq);

        if let Some(colored_piece) = piece_opt {
            let color = colored_piece.color();
            let piece = colored_piece.piece();

            assert_ne!(
                board.occupied() & sq_bb,
                0,
                "Square {sq:?} has mailbox piece {colored_piece:?} but is not occupied in bitboard"
            );
            assert_ne!(
                board.colors(color) & sq_bb,
                0,
                "Square {sq:?} has mailbox color {color:?} but color bitboard bit is unset"
            );
            assert_ne!(
                board.pieces(piece) & sq_bb,
                0,
                "Square {sq:?} has mailbox piece {piece:?} but piece bitboard bit is unset"
            );
        } else {
            assert_eq!(
                board.occupied() & sq_bb,
                0,
                "Square {sq:?} has empty mailbox but occupied bitboard bit is set"
            );
        }
    }

    // 5. Exactly one King per color
    let white_king_bb = board.color_piece(Piece::King, Color::White);
    let black_king_bb = board.color_piece(Piece::King, Color::Black);
    assert_eq!(
        white_king_bb.count_ones(),
        1,
        "White must have exactly one king"
    );
    assert_eq!(
        black_king_bb.count_ones(),
        1,
        "Black must have exactly one king"
    );

    assert_eq!(
        board.king_sq(Color::White).bitboard(),
        white_king_bb,
        "board.king_sq(White) mismatch"
    );
    assert_eq!(
        board.king_sq(Color::Black).bitboard(),
        black_king_bb,
        "board.king_sq(Black) mismatch"
    );

    // 6. En passant target square must be on rank 3 (Black to play) or rank 6 (White to play) and empty
    if let Some(ep_sq) = board.en_passant_target_sq() {
        assert_eq!(
            board.occupied() & ep_sq.bitboard(),
            0,
            "En passant target square {ep_sq:?} must be empty"
        );
        let expected_rank = if board.to_play == Color::White { 5 } else { 2 };
        assert_eq!(
            ep_sq.rank(),
            expected_rank,
            "En passant target square rank mismatch for side to play"
        );
    }
}

#[test]
fn test_quiet_piece_moves() {
    let mut board = Board::start_pos();
    assert_board_invariants(&board);

    // 1. White Knight Nf3 (G1 -> F3)
    let mov = Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::G1), None);
    assert_eq!(
        board.piece_at(Sq::F3),
        Some(ColoredPiece::new(Piece::Knight, Color::White))
    );
    assert_eq!(board.to_play, Color::Black);
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 1);
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_board_invariants(&board);

    // 2. Black Knight Nc6 (B8 -> C6)
    let mov = Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::B8), None);
    assert_eq!(
        board.piece_at(Sq::C6),
        Some(ColoredPiece::new(Piece::Knight, Color::Black))
    );
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.half_move_clock(), 2);
    assert_eq!(board.ply, 2);
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.castling_rights(), CastlingRights::ALL);
    assert_board_invariants(&board);
}

#[test]
fn test_quiet_pawn_pushes() {
    let mut board = Board::start_pos();

    // 1. White single pawn push e2 -> e3
    let mov = Move::new(Sq::E2, Sq::E3, MoveFlags::Quiet);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::E2), None);
    assert_eq!(
        board.piece_at(Sq::E3),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_eq!(board.to_play, Color::Black);
    // Pawn moves reset half_move_clock
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 1);
    // Single push should NOT set en_passant_target_sq
    assert_eq!(board.en_passant_target_sq(), None);
    assert_board_invariants(&board);

    // 2. Black single pawn push d7 -> d6
    let mov = Move::new(Sq::D7, Sq::D6, MoveFlags::Quiet);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::D7), None);
    assert_eq!(
        board.piece_at(Sq::D6),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 2);
    assert_eq!(board.en_passant_target_sq(), None);
    assert_board_invariants(&board);
}

#[test]
fn test_double_pawn_pushes_and_en_passant_target() {
    let mut board = Board::start_pos();

    // White plays e2 -> e4 (DoublePawn) from startpos
    // No Black pawn in position to capture e.p. -> target square is None
    let mov = Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::E2), None);
    assert_eq!(
        board.piece_at(Sq::E4),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 1);
    assert_eq!(board.to_play, Color::Black);
    assert_board_invariants(&board);

    // Black plays d7 -> d5 (DoublePawn)
    // White's pawn is on e4 (not e5), so no e.p. capture is possible
    let mov = Move::new(Sq::D7, Sq::D5, MoveFlags::DoublePawn);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::D7), None);
    assert_eq!(
        board.piece_at(Sq::D5),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 2);
    assert_eq!(board.to_play, Color::White);
    assert_board_invariants(&board);

    // With adjacent Black pawn on d4, White playing e2 -> e4 sets e.p. target
    let fen_with_attacker = "rnbqkbnr/pppppppp/8/8/3p4/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board_with_attacker = Board::from_fen(fen_with_attacker).unwrap();
    board_with_attacker.make_move(Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn));
    assert_eq!(board_with_attacker.en_passant_target_sq(), Some(Sq::E3));

    // Next quiet move resets e.p. target
    board_with_attacker.make_move(Move::new(Sq::G8, Sq::F6, MoveFlags::Quiet));
    assert_eq!(board_with_attacker.en_passant_target_sq(), None);
    assert_board_invariants(&board_with_attacker);
}

fn rank_with_pawn(file: u8, ch: char) -> String {
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
fn test_all_files_double_pawn_push_ep_square() {
    for file in 0..8 {
        // White double push from rank 2 (index 1) to rank 4 (index 3)
        let from_w = Sq::new(file, 1).unwrap();
        let to_w = Sq::new(file, 3).unwrap();
        let expected_ep_w = Sq::new(file, 2).unwrap();

        // Without adjacent Black pawn on rank 4, EP square is None
        let mut board_no_enemy = Board::start_pos();
        board_no_enemy.make_move(Move::new(from_w, to_w, MoveFlags::DoublePawn));
        assert_eq!(
            board_no_enemy.en_passant_target_sq(),
            None,
            "White EP square should be None when no enemy pawn attacks file {file}"
        );

        // With adjacent Black pawn on rank 4 (index 3), EP square is Some
        let adj_file = if file > 0 { file - 1 } else { file + 1 };
        let fen_w = format!(
            "rnbqkbnr/pppppppp/8/8/{}/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            rank_with_pawn(adj_file, 'p')
        );
        let mut board = Board::from_fen(&fen_w).unwrap();
        board.make_move(Move::new(from_w, to_w, MoveFlags::DoublePawn));
        assert_eq!(
            board.en_passant_target_sq(),
            Some(expected_ep_w),
            "White EP square incorrect for file {file}"
        );
        assert_board_invariants(&board);

        // Black double push from rank 7 (index 6) to rank 5 (index 4)
        let from_b = Sq::new(file, 6).unwrap();
        let to_b = Sq::new(file, 4).unwrap();
        let expected_ep_b = Sq::new(file, 5).unwrap();

        // Without adjacent White pawn on rank 5, EP square is None
        let mut b_no_enemy = Board::start_pos();
        b_no_enemy.to_play = Color::Black;
        b_no_enemy.make_move(Move::new(from_b, to_b, MoveFlags::DoublePawn));
        assert_eq!(
            b_no_enemy.en_passant_target_sq(),
            None,
            "Black EP square should be None when no enemy pawn attacks file {file}"
        );

        // With adjacent White pawn on rank 5 (index 4), EP square is Some
        let fen_b = format!(
            "rnbqkbnr/pppppppp/8/{}/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
            rank_with_pawn(adj_file, 'P')
        );
        let mut b = Board::from_fen(&fen_b).unwrap();
        b.make_move(Move::new(from_b, to_b, MoveFlags::DoublePawn));
        assert_eq!(
            b.en_passant_target_sq(),
            Some(expected_ep_b),
            "Black EP square incorrect for file {file}"
        );
        assert_board_invariants(&b);
    }
}

#[test]
fn test_white_en_passant_capture() {
    // Setup position: White Pawn on e5, Black Pawn on d7
    let fen = "rnbqkbnr/pppppppp/8/4P3/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    // Black plays d7 -> d5
    board.make_move(Move::new(Sq::D7, Sq::D5, MoveFlags::DoublePawn));
    assert_eq!(board.en_passant_target_sq(), Some(Sq::D6));
    assert_eq!(
        board.piece_at(Sq::D5),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_board_invariants(&board);

    // White plays e5xd6 e.p.
    let ep_move = Move::new(Sq::E5, Sq::D6, MoveFlags::EnPassant);
    board.make_move(ep_move);

    // White pawn now on d6, e5 is empty, captured black pawn on d5 is gone
    assert_eq!(board.piece_at(Sq::E5), None);
    assert_eq!(
        board.piece_at(Sq::D6),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_eq!(
        board.piece_at(Sq::D5),
        None,
        "Captured pawn at d5 must be removed"
    );

    // Verify bitboards
    assert_eq!(
        board.color_piece(Piece::Pawn, Color::Black) & Sq::D5.bitboard(),
        0
    );
    assert_eq!(
        board.color_piece(Piece::Pawn, Color::White) & Sq::D6.bitboard(),
        Sq::D6.bitboard()
    );

    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.to_play, Color::Black);
    assert_board_invariants(&board);
}

#[test]
fn test_black_en_passant_capture() {
    // Setup position: Black Pawn on e4, White Pawn on f2
    let fen = "rnbqkbnr/pppp1ppp/8/8/4p3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    // White plays f2 -> f4
    board.make_move(Move::new(Sq::F2, Sq::F4, MoveFlags::DoublePawn));
    assert_eq!(board.en_passant_target_sq(), Some(Sq::F3));
    assert_eq!(
        board.piece_at(Sq::F4),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_board_invariants(&board);

    // Black plays e4xf3 e.p.
    let ep_move = Move::new(Sq::E4, Sq::F3, MoveFlags::EnPassant);
    board.make_move(ep_move);

    // Black pawn on f3, e4 is empty, captured white pawn on f4 is gone
    assert_eq!(board.piece_at(Sq::E4), None);
    assert_eq!(
        board.piece_at(Sq::F3),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_eq!(
        board.piece_at(Sq::F4),
        None,
        "Captured pawn at f4 must be removed"
    );

    // Verify bitboards
    assert_eq!(
        board.color_piece(Piece::Pawn, Color::White) & Sq::F4.bitboard(),
        0
    );
    assert_eq!(
        board.color_piece(Piece::Pawn, Color::Black) & Sq::F3.bitboard(),
        Sq::F3.bitboard()
    );

    assert_eq!(board.en_passant_target_sq(), None);
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.to_play, Color::White);
    assert_board_invariants(&board);
}

#[test]
fn test_regular_piece_and_pawn_captures() {
    // Setup position with various pieces ready to capture
    let fen = "r1bqk2r/pppp1ppp/2n5/4p3/1bB1n3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 5";
    let mut board = Board::from_fen(fen).unwrap();

    // 1. White Knight captures Black Knight on e4 (Nxe4)
    let mov = Move::new(Sq::C3, Sq::E4, MoveFlags::Capture);
    board.make_move(mov);

    assert_eq!(board.piece_at(Sq::C3), None);
    assert_eq!(
        board.piece_at(Sq::E4),
        Some(ColoredPiece::new(Piece::Knight, Color::White))
    );
    assert_eq!(
        board.half_move_clock(),
        0,
        "Capture must reset half_move_clock"
    );
    assert_eq!(board.to_play, Color::Black);
    assert_board_invariants(&board);

    // 2. Black Pawn captures White Knight on e4 (d5xe4)
    let fen2 = "r1bqk2r/ppp2ppp/2n5/3pp3/1bB1N3/5N2/PPPP1PPP/R1BQK2R b KQkq - 0 5";
    let mut board2 = Board::from_fen(fen2).unwrap();
    let mov2 = Move::new(Sq::D5, Sq::E4, MoveFlags::Capture);
    board2.make_move(mov2);

    assert_eq!(board2.piece_at(Sq::D5), None);
    assert_eq!(
        board2.piece_at(Sq::E4),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_eq!(board2.half_move_clock(), 0);
    assert_eq!(board2.to_play, Color::White);
    assert_board_invariants(&board2);
}

#[test]
fn test_castling_rights_revocation_on_rook_capture() {
    // 1. White Bishop on a3 captures Black Rook on a8 -> Black loses BLACK_000
    let fen = "r3k2r/8/8/8/8/B7/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();
    assert_eq!(board.castling_rights(), CastlingRights::ALL);

    board.make_move(Move::new(Sq::A3, Sq::A8, MoveFlags::Capture));
    assert_eq!(
        board.castling_rights(),
        CastlingRights::WHITE_ANY | CastlingRights::BLACK_00,
        "Black BLACK_000 right should be revoked when a8 rook is captured"
    );
    assert_board_invariants(&board);

    // 2. White Bishop on h3 captures Black Rook on h8 -> Black loses BLACK_00
    let fen2 = "r3k2r/8/8/8/8/7B/8/R3K2R w KQkq - 0 1";
    let mut board2 = Board::from_fen(fen2).unwrap();
    board2.make_move(Move::new(Sq::H3, Sq::H8, MoveFlags::Capture));
    assert_eq!(
        board2.castling_rights(),
        CastlingRights::WHITE_ANY | CastlingRights::BLACK_000,
        "Black BLACK_00 right should be revoked when h8 rook is captured"
    );
    assert_board_invariants(&board2);

    // 3. Black Bishop on a6 captures White Rook on a1 -> White loses WHITE_000
    let fen3 = "r3k2r/8/b7/8/8/8/8/R3K2R b KQkq - 0 1";
    let mut board3 = Board::from_fen(fen3).unwrap();
    board3.make_move(Move::new(Sq::A6, Sq::A1, MoveFlags::Capture));
    assert_eq!(
        board3.castling_rights(),
        CastlingRights::WHITE_00 | CastlingRights::BLACK_ANY,
        "White WHITE_000 right should be revoked when a1 rook is captured"
    );
    assert_board_invariants(&board3);

    // 4. Black Bishop on h6 captures White Rook on h1 -> White loses WHITE_00
    let fen4 = "r3k2r/8/7b/8/8/8/8/R3K2R b KQkq - 0 1";
    let mut board4 = Board::from_fen(fen4).unwrap();
    board4.make_move(Move::new(Sq::H6, Sq::H1, MoveFlags::Capture));
    assert_eq!(
        board4.castling_rights(),
        CastlingRights::WHITE_000 | CastlingRights::BLACK_ANY,
        "White WHITE_00 right should be revoked when h1 rook is captured"
    );
    assert_board_invariants(&board4);
}

#[test]
fn test_castling_rights_revocation_on_king_and_rook_moves() {
    // 1. White King moves -> White loses all castling rights
    let fen_r = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen_r).unwrap();
    board.make_move(Move::new(Sq::E1, Sq::E2, MoveFlags::Quiet));
    assert_eq!(
        board.castling_rights(),
        CastlingRights::BLACK_ANY,
        "White moving King from e1 must revoke all White castling rights"
    );
    assert_board_invariants(&board);

    // 2. Black King moves -> Black loses all castling rights
    let mut board_b = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1").unwrap();
    board_b.make_move(Move::new(Sq::E8, Sq::E7, MoveFlags::Quiet));
    assert_eq!(
        board_b.castling_rights(),
        CastlingRights::WHITE_ANY,
        "Black moving King from e8 must revoke all Black castling rights"
    );
    assert_board_invariants(&board_b);

    // 3. White Rook moving from a1 revokes WHITE_000
    let mut board2 = Board::from_fen(fen_r).unwrap();
    board2.make_move(Move::new(Sq::A1, Sq::B1, MoveFlags::Quiet));
    assert_eq!(
        board2.castling_rights(),
        CastlingRights::WHITE_00 | CastlingRights::BLACK_ANY,
        "White moving Rook from a1 must revoke WHITE_000"
    );
    assert_board_invariants(&board2);

    // 4. White Rook moving from h1 revokes WHITE_00
    let mut board3 = Board::from_fen(fen_r).unwrap();
    board3.make_move(Move::new(Sq::H1, Sq::G1, MoveFlags::Quiet));
    assert_eq!(
        board3.castling_rights(),
        CastlingRights::WHITE_000 | CastlingRights::BLACK_ANY,
        "White moving Rook from h1 must revoke WHITE_00"
    );
    assert_board_invariants(&board3);

    // 5. Black Rook moving from a8 revokes BLACK_000
    let mut board4 = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1").unwrap();
    board4.make_move(Move::new(Sq::A8, Sq::B8, MoveFlags::Quiet));
    assert_eq!(
        board4.castling_rights(),
        CastlingRights::WHITE_ANY | CastlingRights::BLACK_00,
        "Black moving Rook from a8 must revoke BLACK_000"
    );
    assert_board_invariants(&board4);

    // 6. Black Rook moving from h8 revokes BLACK_00
    let mut board5 = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1").unwrap();
    board5.make_move(Move::new(Sq::H8, Sq::G8, MoveFlags::Quiet));
    assert_eq!(
        board5.castling_rights(),
        CastlingRights::WHITE_ANY | CastlingRights::BLACK_000,
        "Black moving Rook from h8 must revoke BLACK_00"
    );
    assert_board_invariants(&board5);
}

#[test]
fn test_white_king_side_castling() {
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    let castling_move = Move::new(Sq::E1, Sq::G1, MoveFlags::CastleKing);
    board.make_move(castling_move);

    // King moved from e1 to g1
    assert_eq!(board.piece_at(Sq::E1), None);
    assert_eq!(
        board.piece_at(Sq::G1),
        Some(ColoredPiece::new(Piece::King, Color::White))
    );

    // Rook moved from h1 to f1
    assert_eq!(board.piece_at(Sq::H1), None);
    assert_eq!(
        board.piece_at(Sq::F1),
        Some(ColoredPiece::new(Piece::Rook, Color::White))
    );

    // Castling rights for White cleared, Black retained
    assert_eq!(board.castling_rights(), CastlingRights::BLACK_ANY);
    assert_eq!(board.to_play, Color::Black);
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 1);
    assert_board_invariants(&board);
}

#[test]
fn test_white_queen_side_castling() {
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    let castling_move = Move::new(Sq::E1, Sq::C1, MoveFlags::CastleQueen);
    board.make_move(castling_move);

    // King moved from e1 to c1
    assert_eq!(board.piece_at(Sq::E1), None);
    assert_eq!(
        board.piece_at(Sq::C1),
        Some(ColoredPiece::new(Piece::King, Color::White))
    );

    // Rook moved from a1 to d1
    assert_eq!(board.piece_at(Sq::A1), None);
    assert_eq!(
        board.piece_at(Sq::D1),
        Some(ColoredPiece::new(Piece::Rook, Color::White))
    );

    // Castling rights for White cleared, Black retained
    assert_eq!(board.castling_rights(), CastlingRights::BLACK_ANY);
    assert_eq!(board.to_play, Color::Black);
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 1);
    assert_board_invariants(&board);
}

#[test]
fn test_black_king_side_castling() {
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    let castling_move = Move::new(Sq::E8, Sq::G8, MoveFlags::CastleKing);
    board.make_move(castling_move);

    // King moved from e8 to g8
    assert_eq!(board.piece_at(Sq::E8), None);
    assert_eq!(
        board.piece_at(Sq::G8),
        Some(ColoredPiece::new(Piece::King, Color::Black))
    );

    // Rook moved from h8 to f8
    assert_eq!(board.piece_at(Sq::H8), None);
    assert_eq!(
        board.piece_at(Sq::F8),
        Some(ColoredPiece::new(Piece::Rook, Color::Black))
    );

    // Castling rights for Black cleared, White retained
    assert_eq!(board.castling_rights(), CastlingRights::WHITE_ANY);
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 2);
    assert_board_invariants(&board);
}

#[test]
fn test_black_queen_side_castling() {
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();
    assert_board_invariants(&board);

    let castling_move = Move::new(Sq::E8, Sq::C8, MoveFlags::CastleQueen);
    board.make_move(castling_move);

    // King moved from e8 to c8
    assert_eq!(board.piece_at(Sq::E8), None);
    assert_eq!(
        board.piece_at(Sq::C8),
        Some(ColoredPiece::new(Piece::King, Color::Black))
    );

    // Rook moved from a8 to d8
    assert_eq!(board.piece_at(Sq::A8), None);
    assert_eq!(
        board.piece_at(Sq::D8),
        Some(ColoredPiece::new(Piece::Rook, Color::Black))
    );

    // Castling rights for Black cleared, White retained
    assert_eq!(board.castling_rights(), CastlingRights::WHITE_ANY);
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.half_move_clock(), 1);
    assert_board_invariants(&board);
}

#[test]
fn test_white_quiet_promotions() {
    let promotions = [
        (MoveFlags::PromoQueen, Piece::Queen),
        (MoveFlags::PromoRook, Piece::Rook),
        (MoveFlags::PromoBishop, Piece::Bishop),
        (MoveFlags::PromoKnight, Piece::Knight),
    ];

    for (flag, expected_piece) in promotions {
        let fen = "8/4P3/8/8/8/8/8/4K2k w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        assert_board_invariants(&board);

        let promo_move = Move::new(Sq::E7, Sq::E8, flag);
        board.make_move(promo_move);

        assert_eq!(board.piece_at(Sq::E7), None);
        assert_eq!(
            board.piece_at(Sq::E8),
            Some(ColoredPiece::new(expected_piece, Color::White))
        );
        assert_eq!(board.color_piece(Piece::Pawn, Color::White), 0);
        assert_eq!(
            board.color_piece(expected_piece, Color::White) & Sq::E8.bitboard(),
            Sq::E8.bitboard()
        );
        assert_eq!(board.half_move_clock(), 0);
        assert_eq!(board.to_play, Color::Black);
        assert_board_invariants(&board);
    }
}

#[test]
fn test_black_quiet_promotions() {
    let promotions = [
        (MoveFlags::PromoQueen, Piece::Queen),
        (MoveFlags::PromoRook, Piece::Rook),
        (MoveFlags::PromoBishop, Piece::Bishop),
        (MoveFlags::PromoKnight, Piece::Knight),
    ];

    for (flag, expected_piece) in promotions {
        let fen = "4K2k/8/8/8/8/8/4p3/8 b - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        assert_board_invariants(&board);

        let promo_move = Move::new(Sq::E2, Sq::E1, flag);
        board.make_move(promo_move);

        assert_eq!(board.piece_at(Sq::E2), None);
        assert_eq!(
            board.piece_at(Sq::E1),
            Some(ColoredPiece::new(expected_piece, Color::Black))
        );
        assert_eq!(board.color_piece(Piece::Pawn, Color::Black), 0);
        assert_eq!(
            board.color_piece(expected_piece, Color::Black) & Sq::E1.bitboard(),
            Sq::E1.bitboard()
        );
        assert_eq!(board.half_move_clock(), 0);
        assert_eq!(board.to_play, Color::White);
        assert_board_invariants(&board);
    }
}

#[test]
fn test_white_capture_promotions() {
    let promotions = [
        (MoveFlags::PromoCaptureQueen, Piece::Queen),
        (MoveFlags::PromoCaptureRook, Piece::Rook),
        (MoveFlags::PromoCaptureBishop, Piece::Bishop),
        (MoveFlags::PromoCaptureKnight, Piece::Knight),
    ];

    for (flag, expected_piece) in promotions {
        // Black has a rook on d8, White pawn on e7
        let fen = "3r3k/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        assert_board_invariants(&board);

        let promo_move = Move::new(Sq::E7, Sq::D8, flag);
        board.make_move(promo_move);

        assert_eq!(board.piece_at(Sq::E7), None);
        assert_eq!(
            board.piece_at(Sq::D8),
            Some(ColoredPiece::new(expected_piece, Color::White))
        );
        // Captured rook on d8 is removed
        assert_eq!(board.color_piece(Piece::Rook, Color::Black), 0);
        assert_eq!(board.color_piece(Piece::Pawn, Color::White), 0);
        assert_eq!(
            board.color_piece(expected_piece, Color::White) & Sq::D8.bitboard(),
            Sq::D8.bitboard()
        );
        assert_eq!(board.half_move_clock(), 0);
        assert_board_invariants(&board);
    }
}

#[test]
fn test_black_capture_promotions() {
    let promotions = [
        (MoveFlags::PromoCaptureQueen, Piece::Queen),
        (MoveFlags::PromoCaptureRook, Piece::Rook),
        (MoveFlags::PromoCaptureBishop, Piece::Bishop),
        (MoveFlags::PromoCaptureKnight, Piece::Knight),
    ];

    for (flag, expected_piece) in promotions {
        // White has a bishop on c1, Black pawn on d2
        let fen = "4K3/8/8/8/8/8/3p4/2B3k1 b - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        assert_board_invariants(&board);

        let promo_move = Move::new(Sq::D2, Sq::C1, flag);
        board.make_move(promo_move);

        assert_eq!(board.piece_at(Sq::D2), None);
        assert_eq!(
            board.piece_at(Sq::C1),
            Some(ColoredPiece::new(expected_piece, Color::Black))
        );
        // Captured bishop on c1 is removed
        assert_eq!(board.color_piece(Piece::Bishop, Color::White), 0);
        assert_eq!(board.color_piece(Piece::Pawn, Color::Black), 0);
        assert_eq!(
            board.color_piece(expected_piece, Color::Black) & Sq::C1.bitboard(),
            Sq::C1.bitboard()
        );
        assert_eq!(board.half_move_clock(), 0);
        assert_board_invariants(&board);
    }
}

#[test]
fn test_promotion_capture_corner_rook_castling_rights() {
    // White pawn on b7 captures Black rook on a8 promoting to Queen
    let fen = "r3k2r/1P6/8/8/8/8/8/4K2R w Kkq - 0 1";
    let mut board = Board::from_fen(fen).unwrap();

    let promo_move = Move::new(Sq::B7, Sq::A8, MoveFlags::PromoCaptureQueen);
    board.make_move(promo_move);

    assert_eq!(
        board.piece_at(Sq::A8),
        Some(ColoredPiece::new(Piece::Queen, Color::White))
    );
    assert_eq!(
        board.castling_rights(),
        CastlingRights::WHITE_00 | CastlingRights::BLACK_00,
        "Capturing rook on a8 with promotion must revoke BLACK_000"
    );
    assert_board_invariants(&board);
}

#[test]
fn test_half_move_clock_and_ply_progression() {
    let mut board = Board::start_pos();
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 0);

    // 1. e4 (double pawn push) -> reset
    board.make_move(Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn));
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 1);

    // 1... e5 (double pawn push) -> reset
    board.make_move(Move::new(Sq::E7, Sq::E5, MoveFlags::DoublePawn));
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 2);

    // 2. Nf3 (quiet knight) -> clock = 1
    board.make_move(Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet));
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 3);

    // 2... Nc6 (quiet knight) -> clock = 2
    board.make_move(Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet));
    assert_eq!(board.half_move_clock(), 2);
    assert_eq!(board.ply, 4);

    // 3. Bb5 (quiet bishop) -> clock = 3
    board.make_move(Move::new(Sq::F1, Sq::B5, MoveFlags::Quiet));
    assert_eq!(board.half_move_clock(), 3);
    assert_eq!(board.ply, 5);

    // 3... a6 (single pawn push) -> reset
    board.make_move(Move::new(Sq::A7, Sq::A6, MoveFlags::Quiet));
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 6);

    // 4. Bxc6 (bishop capture) -> reset
    board.make_move(Move::new(Sq::B5, Sq::C6, MoveFlags::Capture));
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 7);

    // 4... dxc6 (pawn capture) -> reset
    board.make_move(Move::new(Sq::D7, Sq::C6, MoveFlags::Capture));
    assert_eq!(board.half_move_clock(), 0);
    assert_eq!(board.ply, 8);

    // 5. O-O (king-side castle) -> clock = 1
    board.make_move(Move::new(Sq::E1, Sq::G1, MoveFlags::CastleKing));
    assert_eq!(board.half_move_clock(), 1);
    assert_eq!(board.ply, 9);

    // 5... Bd6 (quiet bishop) -> clock = 2
    board.make_move(Move::new(Sq::F8, Sq::D6, MoveFlags::Quiet));
    assert_eq!(board.half_move_clock(), 2);
    assert_eq!(board.ply, 10);

    assert_board_invariants(&board);
}

#[test]
fn test_scholars_mate_sequence() {
    let mut board = Board::start_pos();

    // 1. e4 e5
    board.make_move(Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn));
    board.make_move(Move::new(Sq::E7, Sq::E5, MoveFlags::DoublePawn));

    // 2. Qh5 Nc6
    board.make_move(Move::new(Sq::D1, Sq::H5, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet));

    // 3. Bc4 Nf6
    board.make_move(Move::new(Sq::F1, Sq::C4, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::G8, Sq::F6, MoveFlags::Quiet));

    // 4. Qxf7#
    board.make_move(Move::new(Sq::H5, Sq::F7, MoveFlags::Capture));

    assert_eq!(
        board.piece_at(Sq::F7),
        Some(ColoredPiece::new(Piece::Queen, Color::White))
    );
    assert_eq!(board.piece_at(Sq::H5), None);
    assert_eq!(board.to_play, Color::Black);
    assert_eq!(board.ply, 7);
    assert_eq!(board.half_move_clock(), 0);
    assert_board_invariants(&board);

    // Compare with direct FEN parse
    let fen_expected = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
    let board_expected = Board::from_fen(fen_expected).unwrap();

    assert_eq!(board.occupied(), board_expected.occupied());
    assert_eq!(
        board.colors(Color::White),
        board_expected.colors(Color::White)
    );
    assert_eq!(
        board.colors(Color::Black),
        board_expected.colors(Color::Black)
    );
    assert_eq!(board.castling_rights(), board_expected.castling_rights());
    assert_eq!(board.to_play, board_expected.to_play);
    assert_eq!(board.half_move_clock(), board_expected.half_move_clock());
    assert_eq!(board.ply, board_expected.ply);
}

#[test]
fn test_fools_mate_sequence() {
    let mut board = Board::start_pos();

    // 1. f3 e5
    board.make_move(Move::new(Sq::F2, Sq::F3, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::E7, Sq::E5, MoveFlags::DoublePawn));

    // 2. g4 Qh4#
    board.make_move(Move::new(Sq::G2, Sq::G4, MoveFlags::DoublePawn));
    board.make_move(Move::new(Sq::D8, Sq::H4, MoveFlags::Quiet));

    assert_eq!(
        board.piece_at(Sq::H4),
        Some(ColoredPiece::new(Piece::Queen, Color::Black))
    );
    assert_eq!(board.to_play, Color::White);
    assert_eq!(board.ply, 4);
    assert_board_invariants(&board);

    let fen_expected = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
    let board_expected = Board::from_fen(fen_expected).unwrap();

    assert_eq!(board.occupied(), board_expected.occupied());
    assert_eq!(board.castling_rights(), board_expected.castling_rights());
    assert_eq!(board.to_play, board_expected.to_play);
    assert_eq!(board.ply, board_expected.ply);
}

#[test]
fn test_perft_move_generation_invariants() {
    // Generate moves and apply make_move on a few diverse positions,
    // asserting invariants on all resulting boards.
    use chess_engine::move_gen::{Black, Evasions, MoveList, NonEvasions, White, generate_moves};

    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];

    for fen in fens {
        let board = Board::from_fen(fen).unwrap();
        assert_board_invariants(&board);

        let us = board.to_play;
        let king_sq = board.king_sq(us);
        let in_check = board.generate_attackers(king_sq, !us, board.occupied()) != 0;

        let mut moves = MoveList::default();
        let ptr = match (us, in_check) {
            (Color::White, true) => generate_moves::<White, Evasions>(&board, moves.as_ptr()),
            (Color::White, false) => generate_moves::<White, NonEvasions>(&board, moves.as_ptr()),
            (Color::Black, true) => generate_moves::<Black, Evasions>(&board, moves.as_ptr()),
            (Color::Black, false) => generate_moves::<Black, NonEvasions>(&board, moves.as_ptr()),
        };
        moves.update_size(ptr);

        for scored_move in moves.as_slice() {
            let mov = scored_move.mov;
            if board.legal(mov) {
                let mut child = board.clone();
                child.make_move(mov);
                assert_board_invariants(&child);
            }
        }
    }
}

#[test]
fn test_ruy_lopez_opening_sequence() {
    let mut board = Board::start_pos();

    // 1. e4 e5
    board.make_move(Move::new(Sq::E2, Sq::E4, MoveFlags::DoublePawn));
    board.make_move(Move::new(Sq::E7, Sq::E5, MoveFlags::DoublePawn));

    // 2. Nf3 Nc6
    board.make_move(Move::new(Sq::G1, Sq::F3, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::B8, Sq::C6, MoveFlags::Quiet));

    // 3. Bb5 a6
    board.make_move(Move::new(Sq::F1, Sq::B5, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::A7, Sq::A6, MoveFlags::Quiet));

    // 4. Ba4 Nf6
    board.make_move(Move::new(Sq::B5, Sq::A4, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::G8, Sq::F6, MoveFlags::Quiet));

    // 5. O-O Be7
    board.make_move(Move::new(Sq::E1, Sq::G1, MoveFlags::CastleKing));
    board.make_move(Move::new(Sq::F8, Sq::E7, MoveFlags::Quiet));

    // 6. Re1 b5
    board.make_move(Move::new(Sq::F1, Sq::E1, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::B7, Sq::B5, MoveFlags::DoublePawn));

    // 7. Bb3 d6
    board.make_move(Move::new(Sq::A4, Sq::B3, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::D7, Sq::D6, MoveFlags::Quiet));

    // 8. c3 O-O
    board.make_move(Move::new(Sq::C2, Sq::C3, MoveFlags::Quiet));
    board.make_move(Move::new(Sq::E8, Sq::G8, MoveFlags::CastleKing));

    assert_board_invariants(&board);

    let fen_expected = "r1bq1rk1/2p1bppp/p1np1n2/1p2p3/4P3/1BP2N2/PP1P1PPP/RNBQR1K1 w - - 1 9";
    let board_expected = Board::from_fen(fen_expected).unwrap();

    assert_eq!(board.occupied(), board_expected.occupied());
    assert_eq!(
        board.colors(Color::White),
        board_expected.colors(Color::White)
    );
    assert_eq!(
        board.colors(Color::Black),
        board_expected.colors(Color::Black)
    );
    assert_eq!(board.castling_rights(), board_expected.castling_rights());
    assert_eq!(board.to_play, board_expected.to_play);
    assert_eq!(board.half_move_clock(), board_expected.half_move_clock());
    assert_eq!(board.ply, board_expected.ply);
}

#[test]
fn test_edge_files_en_passant() {
    // 1. A-file EP: White pawn on a5, Black plays b7-b5 -> a5xb6 e.p.
    let fen_a = "rnbqkbnr/1ppppppp/8/P7/8/8/1PPPPPPP/RNBQKBNR b KQkq - 0 2";
    let mut board_a = Board::from_fen(fen_a).unwrap();
    board_a.make_move(Move::new(Sq::B7, Sq::B5, MoveFlags::DoublePawn));
    assert_eq!(board_a.en_passant_target_sq(), Some(Sq::B6));

    board_a.make_move(Move::new(Sq::A5, Sq::B6, MoveFlags::EnPassant));
    assert_eq!(board_a.piece_at(Sq::A5), None);
    assert_eq!(board_a.piece_at(Sq::B5), None, "Captured b5 pawn removed");
    assert_eq!(
        board_a.piece_at(Sq::B6),
        Some(ColoredPiece::new(Piece::Pawn, Color::White))
    );
    assert_board_invariants(&board_a);

    // 2. H-file EP: Black pawn on h4, White plays g2-g4 -> h4xg3 e.p.
    let fen_h = "rnbqkbnr/pppppp1p/8/8/7p/8/PPPPPPP1/RNBQKBNR w KQkq - 0 2";
    let mut board_h = Board::from_fen(fen_h).unwrap();
    board_h.make_move(Move::new(Sq::G2, Sq::G4, MoveFlags::DoublePawn));
    assert_eq!(board_h.en_passant_target_sq(), Some(Sq::G3));

    board_h.make_move(Move::new(Sq::H4, Sq::G3, MoveFlags::EnPassant));
    assert_eq!(board_h.piece_at(Sq::H4), None);
    assert_eq!(board_h.piece_at(Sq::G4), None, "Captured g4 pawn removed");
    assert_eq!(
        board_h.piece_at(Sq::G3),
        Some(ColoredPiece::new(Piece::Pawn, Color::Black))
    );
    assert_board_invariants(&board_h);
}
