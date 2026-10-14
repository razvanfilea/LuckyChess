use crate::common::bb;
use chess_core::bitboard::*;
use chess_core::prelude::*;
use chess_engine::attacks::*;

fn all_squares() -> impl Iterator<Item = Sq> {
    (0..Sq::NB as u8).map(|i| Sq::from_raw(i).unwrap())
}

fn offset(sq: Sq, df: i8, dr: i8) -> Option<Sq> {
    Sq::new(
        sq.file().wrapping_add_signed(df),
        sq.rank().wrapping_add_signed(dr),
    )
}

/// Reference attacks of a leaper, built from its (file, rank) offsets
fn leaper(sq: Sq, deltas: &[(i8, i8)]) -> u64 {
    deltas
        .iter()
        .filter_map(|&(df, dr)| offset(sq, df, dr))
        .fold(0, |acc, to| acc | to.bitboard())
}

const fn delta(dir: Dir) -> (i8, i8) {
    match dir {
        Dir::North => (0, 1),
        Dir::South => (0, -1),
        Dir::East => (1, 0),
        Dir::West => (-1, 0),
        Dir::NorthEast => (1, 1),
        Dir::NorthWest => (-1, 1),
        Dir::SouthEast => (1, -1),
        Dir::SouthWest => (-1, -1),
    }
}

#[test]
fn test_square_coordinates() {
    for (i, sq) in all_squares().enumerate() {
        assert_eq!(sq as usize, i);
        assert_eq!(sq.bitboard(), 1 << i);
        assert_eq!(Sq::new(sq.file(), sq.rank()), Some(sq));
        assert_eq!(Sq::parse(&sq.to_string()), Some(sq));
        assert_eq!(format!("{sq:?}"), sq.to_string());
    }
    assert_eq!(Sq::E4.to_string(), "e4");
    assert_eq!((Sq::E4.file(), Sq::E4.rank()), (4, 3));

    assert_eq!(Sq::new(8, 0), None);
    assert_eq!(Sq::new(0, 8), None);
    assert_eq!(Sq::from_raw(64), None);
    for bad in ["", "a", "i1", "a9", "H8"] {
        assert_eq!(Sq::parse(bad), None, "{bad:?}");
    }
    assert_eq!(Sq::parse("a1x"), Some(Sq::A1));
}

#[test]
fn test_square_distances() {
    assert_eq!(Sq::A1.distance_to(Sq::H8), 7);
    assert_eq!(Sq::A1.distance_to(Sq::B2), 1);
    assert_eq!(Sq::E4.distance_to(Sq::E4), 0);
    assert_eq!(Sq::A1.manhattan_distance_to(Sq::H8), 14);
    assert_eq!(Sq::A1.manhattan_distance_to(Sq::B2), 2);
    assert_eq!(Sq::A1.distance_to_file_edge(), 0);
    assert_eq!(Sq::H8.distance_to_rank_edge(), 0);
    assert_eq!(Sq::E4.distance_to_file_edge(), 3);
    assert_eq!(Sq::D5.distance_to_rank_edge(), 3);
}

#[test]
fn test_pieces_and_colors() {
    for cp in ColoredPiece::ALL {
        assert_eq!(ColoredPiece::new(cp.piece(), cp.color()), cp);
        assert_eq!(ColoredPiece::parse(cp.to_char()), Some(cp));
        assert_eq!(
            cp.to_char().is_ascii_uppercase(),
            cp.color() == Color::White
        );
        assert!(cp == cp.piece());
    }
    assert_eq!(
        ColoredPiece::parse('n'),
        Some(ColoredPiece::new(Piece::Knight, Color::Black))
    );
    assert_eq!(ColoredPiece::parse('X'), None);

    assert_eq!(!Color::White, Color::Black);
    assert_eq!(Color::from(true), Color::White);
    assert!(!Color::Black.as_bool());
}

#[test]
fn test_move_encoding() {
    use MoveFlags::*;
    // (flags, capture, castle, promotion)
    let cases = [
        (Quiet, false, false, None),
        (DoublePawn, false, false, None),
        (CastleKing, false, true, None),
        (CastleQueen, false, true, None),
        (Capture, true, false, None),
        (EnPassant, true, false, None),
        (PromoKnight, false, false, Some(Piece::Knight)),
        (PromoBishop, false, false, Some(Piece::Bishop)),
        (PromoRook, false, false, Some(Piece::Rook)),
        (PromoQueen, false, false, Some(Piece::Queen)),
        (PromoCaptureKnight, true, false, Some(Piece::Knight)),
        (PromoCaptureBishop, true, false, Some(Piece::Bishop)),
        (PromoCaptureRook, true, false, Some(Piece::Rook)),
        (PromoCaptureQueen, true, false, Some(Piece::Queen)),
    ];
    for (flags, capture, castle, promotion) in cases {
        let mov = Move::new(Sq::B7, Sq::H8, flags);
        assert_eq!((mov.from(), mov.to(), mov.flags()), (Sq::B7, Sq::H8, flags));
        assert_eq!(mov.is_capture(), capture, "{flags:?}");
        assert_eq!(mov.is_castle(), castle, "{flags:?}");
        assert_eq!(mov.is_promotion(), promotion.is_some(), "{flags:?}");
        assert_eq!(mov.promotion_piece(), promotion, "{flags:?}");
        assert!(!mov.is_none());
    }
    assert_eq!(Move::NONE, Move::default());
    assert!(Move::NONE.is_none());

    // The captured en passant pawn sits behind the target square
    let ep = Move::new(Sq::E5, Sq::D6, EnPassant);
    assert_eq!(ep.capture_square(Color::White), Sq::D5);
    let ep = Move::new(Sq::E4, Sq::D3, EnPassant);
    assert_eq!(ep.capture_square(Color::Black), Sq::D4);
}

#[test]
fn test_files_ranks_and_square_colors() {
    let files = [
        FILE_A, FILE_B, FILE_C, FILE_D, FILE_E, FILE_F, FILE_G, FILE_H,
    ];
    let ranks = [
        RANK_1, RANK_2, RANK_3, RANK_4, RANK_5, RANK_6, RANK_7, RANK_8,
    ];
    for sq in all_squares() {
        let bit = sq.bitboard();
        for i in 0..8 {
            assert_eq!(files[i] & bit != 0, sq.file() as usize == i);
            assert_eq!(ranks[i] & bit != 0, sq.rank() as usize == i);
            assert_eq!(bb_file(i as u8), files[i]);
            assert_eq!(bb_rank(i as u8), ranks[i]);
        }
        let light = (sq.file() + sq.rank()) % 2 == 1;
        assert_eq!(LIGHT_SQUARES & bit != 0, light, "{sq}");
        assert_eq!(DARK_SQUARES & bit != 0, !light, "{sq}");
    }
}

#[test]
fn test_directions_and_rays() {
    for dir in Dir::ALL {
        let (df, dr) = delta(dir);
        assert_eq!(delta(dir.opposite()), (-df, -dr));
        assert_eq!(dir.is_forwards(), dr > 0 || (dr == 0 && df > 0));

        for sq in all_squares() {
            let next = offset(sq, df, dr);
            assert_eq!(
                sh_dir(dir, sq.bitboard()),
                next.map_or(0, Sq::bitboard),
                "{dir:?} from {sq}"
            );
            if let Some(next) = next {
                assert_eq!(unsafe { sq.shift(dir) }, next);
            }

            let mut ray = 0;
            let mut cur = sq;
            while let Some(next) = offset(cur, df, dr) {
                ray |= next.bitboard();
                cur = next;
            }
            assert_eq!(bb_from_dir(dir, sq), ray, "{dir:?} from {sq}");
        }
    }
}

#[test]
fn test_between_and_line() {
    for a in all_squares() {
        for b in all_squares() {
            let dir = Dir::ALL
                .into_iter()
                .find(|&dir| bb_from_dir(dir, a) & b.bitboard() != 0);
            let (between, line) = match dir {
                Some(dir) => (
                    bb_from_dir(dir, a) & bb_from_dir(dir.opposite(), b),
                    bb_from_dir(dir, a) | bb_from_dir(dir.opposite(), a) | a.bitboard(),
                ),
                None => (0, 0),
            };
            assert_eq!(bb_between(a, b), between, "between {a} {b}");
            assert_eq!(bb_line(a, b), line, "line {a} {b}");
        }
    }
}

#[test]
fn test_bitboard_scans() {
    let mut set = bb(&[Sq::B1, Sq::E4, Sq::H8]);
    assert_eq!(unsafe { bb_lsb(set) }, Sq::B1);
    assert_eq!(unsafe { bb_msb(set) }, Sq::H8);
    assert_eq!(bb_msb_opt(set), Some(Sq::H8));
    assert_eq!(bb_pop_lsb_opt(&mut set), Some(Sq::B1));
    assert_eq!(unsafe { bb_pop_lsb(&mut set) }, Sq::E4);
    assert_eq!(set, Sq::H8.bitboard());
    assert_eq!(bb_pop_lsb_opt(&mut 0), None);
    assert_eq!(bb_lsb_opt(0), None);
    assert_eq!(bb_msb_opt(0), None);

    assert!(bb_several(0b11) && !bb_several(0b10) && !bb_several(0));
    assert!(bb_only_one(0b10) && !bb_only_one(0b11) && !bb_only_one(0));
}

#[test]
fn test_leaper_attacks() {
    const KNIGHT: [(i8, i8); 8] = [
        (1, 2),
        (2, 1),
        (2, -1),
        (1, -2),
        (-1, -2),
        (-2, -1),
        (-2, 1),
        (-1, 2),
    ];
    const KING: [(i8, i8); 8] = [
        (0, 1),
        (1, 1),
        (1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
        (-1, 0),
        (-1, 1),
    ];
    for sq in all_squares() {
        assert_eq!(knight_attacks(sq), leaper(sq, &KNIGHT), "knight {sq}");
        assert_eq!(king_attacks(sq), leaper(sq, &KING), "king {sq}");
        assert_eq!(
            pawn_attacks(sq, Color::White),
            leaper(sq, &[(-1, 1), (1, 1)]),
            "white pawn {sq}"
        );
        assert_eq!(
            pawn_attacks(sq, Color::Black),
            leaper(sq, &[(-1, -1), (1, -1)]),
            "black pawn {sq}"
        );
    }
}

#[test]
fn test_reference_slider_attacks() {
    let blockers = bb(&[Sq::E2, Sq::E7, Sq::C4, Sq::G4]);
    assert_eq!(
        bb_rook_attacks(Sq::E4, blockers),
        bb(&[
            Sq::E5,
            Sq::E6,
            Sq::E7,
            Sq::E3,
            Sq::E2,
            Sq::F4,
            Sq::G4,
            Sq::D4,
            Sq::C4
        ])
    );

    let blockers = bb(&[Sq::F6, Sq::B6, Sq::F2, Sq::B2]);
    assert_eq!(
        bb_bishop_attacks(Sq::D4, blockers),
        bb(&[
            Sq::E5,
            Sq::F6,
            Sq::C5,
            Sq::B6,
            Sq::E3,
            Sq::F2,
            Sq::C3,
            Sq::B2
        ])
    );
}

#[test]
fn test_slider_attacks_match_reference() {
    let occupancies: [u64; 4] = [
        0,
        0x00FF_00FF_00FF_00FF,
        0xFF00_FF00_FF00_FF00,
        0x8142_2418_1824_4281,
    ];

    for sq in all_squares() {
        assert_eq!(bishop_xray_attacks(sq), bb_bishop_attacks(sq, 0));
        assert_eq!(rook_xray_attacks(sq), bb_rook_attacks(sq, 0));
        for occ in occupancies {
            let bishop = bb_bishop_attacks(sq, occ);
            let rook = bb_rook_attacks(sq, occ);
            assert_eq!(bishop_attacks(sq, occ), bishop, "bishop {sq} {occ:#x}");
            assert_eq!(rook_attacks(sq, occ), rook, "rook {sq} {occ:#x}");
            assert_eq!(queen_attacks(sq, occ), bishop | rook, "queen {sq} {occ:#x}");
        }
    }
}
