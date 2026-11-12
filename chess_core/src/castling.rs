use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use crate::Sq;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const WHITE_00: Self = Self(1 << 0);
    pub const WHITE_000: Self = Self(1 << 1);
    pub const BLACK_00: Self = Self(1 << 2);
    pub const BLACK_000: Self = Self(1 << 3);

    pub const WHITE_ANY: Self = Self(Self::WHITE_00.0 | Self::WHITE_000.0);
    pub const BLACK_ANY: Self = Self(Self::BLACK_00.0 | Self::BLACK_000.0);
    pub const ALL: Self = Self(Self::WHITE_ANY.0 | Self::BLACK_ANY.0);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns true if every right in `other` is also in `self`.
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline(always)]
    pub const fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    #[inline(always)]
    pub const fn mask_for_move(from: Sq, to: Sq) -> CastlingRights {
        Self(CASTLING_RIGHTS_MASKS[from as usize].0 & CASTLING_RIGHTS_MASKS[to as usize].0)
    }
}

static CASTLING_RIGHTS_MASKS: [CastlingRights; Sq::NB] = {
    let mut masks = [CastlingRights::ALL; Sq::NB];
    masks[Sq::A1 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::WHITE_000.0);
    masks[Sq::E1 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::WHITE_ANY.0);
    masks[Sq::H1 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::WHITE_00.0);
    masks[Sq::A8 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::BLACK_000.0);
    masks[Sq::E8 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::BLACK_ANY.0);
    masks[Sq::H8 as usize] = CastlingRights(CastlingRights::ALL.0 & !CastlingRights::BLACK_00.0);
    masks
};

impl BitOr for CastlingRights {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CastlingRights {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CastlingRights {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CastlingRights {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl fmt::Debug for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("-");
        }
        for (right, ch) in [
            (Self::WHITE_00, 'K'),
            (Self::WHITE_000, 'Q'),
            (Self::BLACK_00, 'k'),
            (Self::BLACK_000, 'q'),
        ] {
            if self.contains(right) {
                write!(f, "{ch}")?;
            }
        }
        Ok(())
    }
}
