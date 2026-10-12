use std::hint::unreachable_unchecked;

use crate::{Color, Piece, Sq};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Move(u16);

impl Move {
    pub const NONE: Self = Self(0);

    #[inline(always)]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn new(from: Sq, to: Sq, flags: MoveFlags) -> Self {
        Self(from as u16 | ((to as u16) << 6) | ((flags as u16) << 12))
    }

    #[inline(always)]
    pub const fn from(self) -> Sq {
        unsafe { Sq::from_raw_unchecked((self.0 & 63) as u8) }
    }

    #[inline(always)]
    pub const fn to(self) -> Sq {
        unsafe { Sq::from_raw_unchecked(((self.0 >> 6) & 63) as u8) }
    }

    #[inline(always)]
    pub const fn castling_rook_squares(self, color: Color) -> (Sq, Sq) {
        self.flags().castling_rook_squares(color)
    }

    #[inline(always)]
    pub const fn capture_square(self, color: Color) -> Sq {
        self.flags().capture_square(self.to(), color)
    }

    #[inline(always)]
    pub const fn flags(self) -> MoveFlags {
        unsafe { std::mem::transmute(self.flags_bits()) }
    }

    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        (self.flags_bits() & 0b0100) != 0
    }

    #[inline(always)]
    pub const fn is_promotion(self) -> bool {
        (self.flags_bits() & 0b1000) != 0
    }

    #[inline(always)]
    pub const fn is_castle(self) -> bool {
        !self.is_promotion() && (self.flags_bits() & 0b0010) != 0
    }

    #[inline(always)]
    pub const fn is_tactical(self) -> bool {
        self.is_capture() || self.is_promotion()
    }

    #[inline(always)]
    pub const fn is_quiet(self) -> bool {
        !self.is_tactical()
    }

    #[inline(always)]
    pub const fn promotion_piece(self) -> Option<Piece> {
        if self.is_promotion() {
            match self.flags_bits() & 0b0011 {
                0 => Some(Piece::Knight),
                1 => Some(Piece::Bishop),
                2 => Some(Piece::Rook),
                3 => Some(Piece::Queen),
                _ => unsafe {
                    debug_assert!(false, "unreachable promotion bits");
                    unreachable_unchecked()
                },
            }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn is_any_of(self, other: &[Self; 2]) -> bool {
        self == other[0] || self == other[1]
    }

    #[inline(always)]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Creates a `Move` from raw 16-bit encoding, validating that flag bits are valid.
    #[inline(always)]
    pub const fn from_bits(val: u16) -> Option<Self> {
        let flags = (val >> 12) as u8;
        if flags == 6 || flags == 7 {
            None
        } else {
            Some(Self(val))
        }
    }

    /// Creates a `Move` from raw 16-bit encoding.
    ///
    /// # Safety
    /// The caller must ensure that `val` has valid square and flag bits.
    #[inline(always)]
    pub const unsafe fn from_bits_unchecked(val: u16) -> Self {
        Self(val)
    }

    #[inline(always)]
    const fn flags_bits(self) -> u8 {
        (self.0 >> 12) as u8
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MoveFlags {
    #[default]
    Quiet = 0b0000,
    DoublePawn = 0b0001,
    CastleKing = 0b0010,
    CastleQueen = 0b0011,
    Capture = 0b0100,
    EnPassant = 0b0101,
    PromoKnight = 0b1000,
    PromoBishop = 0b1001,
    PromoRook = 0b1010,
    PromoQueen = 0b1011,
    PromoCaptureKnight = 0b1100,
    PromoCaptureBishop = 0b1101,
    PromoCaptureRook = 0b1110,
    PromoCaptureQueen = 0b1111,
}

impl MoveFlags {
    #[inline(always)]
    pub const fn castling_rook_squares(self, color: Color) -> (Sq, Sq) {
        match (color, self) {
            (Color::White, MoveFlags::CastleKing) => (Sq::H1, Sq::F1),
            (Color::White, _) => (Sq::A1, Sq::D1),
            (Color::Black, MoveFlags::CastleKing) => (Sq::H8, Sq::F8),
            (Color::Black, _) => (Sq::A8, Sq::D8),
        }
    }

    #[inline(always)]
    pub const fn castling_king_path(self, color: Color) -> [Sq; 3] {
        match (color, self) {
            (Color::White, MoveFlags::CastleKing) => [Sq::E1, Sq::F1, Sq::G1],
            (Color::White, _) => [Sq::E1, Sq::D1, Sq::C1],
            (Color::Black, MoveFlags::CastleKing) => [Sq::E8, Sq::F8, Sq::G8],
            (Color::Black, _) => [Sq::E8, Sq::D8, Sq::C8],
        }
    }

    #[inline(always)]
    pub const fn capture_square(self, to: Sq, color: Color) -> Sq {
        match self {
            // SAFETY: An en-passant capture can only land on rank 6 (for White)
            // or rank 3 (for Black). Shifting one square backward is guaranteed
            // to stay within board bounds (rank 5 or 4 respectively).
            MoveFlags::EnPassant => unsafe { to.shift(color.backward()) },
            _ => to,
        }
    }
}
