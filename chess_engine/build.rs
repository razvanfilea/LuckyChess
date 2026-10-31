use chess_core::bitboard::{bb_bishop_attacks, bb_rook_attacks};
use chess_core::piece_tables::{BISHOP_RAYS, ROOK_RAYS};
use chess_core::prng::Prng;
use chess_core::{bitboard, for_each_square, for_subsets, prelude::*};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const TABLE_CAPACITY: usize = 107_648;

#[derive(Clone, Copy)]
enum Slider {
    Rook,
    Bishop,
}

impl Slider {
    fn mask(self, sq: Sq) -> u64 {
        let rays = match self {
            Slider::Rook => ROOK_RAYS[sq as usize],
            Slider::Bishop => BISHOP_RAYS[sq as usize],
        };
        rays & !bitboard::bb_get_edge_filter(sq)
    }

    fn attacks(self, sq: Sq, blockers: u64) -> u64 {
        match self {
            Slider::Rook => bb_rook_attacks(sq, blockers),
            Slider::Bishop => bb_bishop_attacks(sq, blockers),
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
#[allow(dead_code)]
struct SMagic {
    mask: u64,
    magic: u64,
    offset: u32,
    shift: u8,
}

/// Software emulation of the BMI2 `PDEP` instruction for build-time table precomputation.
fn const_pdep(mut val: u64, mut mask: u64) -> u64 {
    let mut res = 0u64;
    let mut bb = 1u64;
    while mask != 0 {
        if (mask & 1) != 0 {
            if (val & 1) != 0 {
                res |= bb;
            }
            val >>= 1;
        }
        mask >>= 1;
        bb <<= 1;
    }
    res
}

fn generate_pext_data(out_dir: &str) {
    let dest_path = Path::new(out_dir).join("pext_data.rs");
    let mut f = File::create(&dest_path).unwrap();

    let mut table = Vec::with_capacity(TABLE_CAPACITY);
    let mut rook_offsets = [0u32; 64];
    let mut bishop_offsets = [0u32; 64];
    let mut rook_masks = [0u64; 64];
    let mut bishop_masks = [0u64; 64];

    for_each_square!(sq => {
        let mask = Slider::Rook.mask(sq);
        rook_offsets[sq as usize] = table.len() as u32;
        rook_masks[sq as usize] = mask;
        let combinations = 1usize << mask.count_ones();
        for j in 0..combinations {
            let occupied = const_pdep(j as u64, mask);
            table.push(Slider::Rook.attacks(sq, occupied));
        }
    });

    for_each_square!(sq => {
        let mask = Slider::Bishop.mask(sq);
        bishop_offsets[sq as usize] = table.len() as u32;
        bishop_masks[sq as usize] = mask;
        let combinations = 1usize << mask.count_ones();
        for j in 0..combinations {
            let occupied = const_pdep(j as u64, mask);
            table.push(Slider::Bishop.attacks(sq, occupied));
        }
    });

    writeln!(
        f,
        "pub static PEXT_TABLE: [u64; {}] = {:?};",
        table.len(),
        table
    )
    .unwrap();
    writeln!(
        f,
        "pub const PEXT_BISHOP_OFFSETS: [u32; 64] = {:?};",
        bishop_offsets
    )
    .unwrap();
    writeln!(
        f,
        "pub const PEXT_ROOK_OFFSETS: [u32; 64] = {:?};",
        rook_offsets
    )
    .unwrap();
    writeln!(
        f,
        "pub const PEXT_BISHOP_MASKS: [u64; 64] = {:?};",
        bishop_masks
    )
    .unwrap();
    writeln!(
        f,
        "pub const PEXT_ROOK_MASKS: [u64; 64] = {:?};",
        rook_masks
    )
    .unwrap();
}

fn find_magic(
    slider: Slider,
    sq: Sq,
    table_offset: &mut usize,
    flat_table: &mut Vec<u64>,
    rng: &mut Prng,
) -> SMagic {
    let mask = slider.mask(sq);
    let relevant_bits = mask.count_ones() as u8;
    let shift = 64 - relevant_bits;
    let size = 1usize << relevant_bits;

    let mut occupancies = Vec::with_capacity(size);
    let mut reference_attacks = Vec::with_capacity(size);

    for_subsets!(occ in mask => {
        occupancies.push(occ);
        reference_attacks.push(slider.attacks(sq, occ));
    });

    let offset = *table_offset as u32;
    *table_offset += size;

    loop {
        let magic = rng.candidate();
        if (mask.wrapping_mul(magic) & 0xFF00_0000_0000_0000).count_ones() < 6 {
            continue;
        }

        let mut used = vec![0u64; size];
        let mut occupied_slots = vec![false; size];
        let mut fail = false;

        for (i, &occ) in occupancies.iter().enumerate() {
            let idx = ((occ.wrapping_mul(magic)) >> shift) as usize;
            if !occupied_slots[idx] {
                occupied_slots[idx] = true;
                used[idx] = reference_attacks[i];
            } else if used[idx] != reference_attacks[i] {
                fail = true;
                break;
            }
        }

        if !fail {
            flat_table.extend_from_slice(&used);
            return SMagic {
                mask,
                magic,
                offset,
                shift,
            };
        }
    }
}

fn generate_magic_data(out_dir: &str) {
    let dest_path = Path::new(out_dir).join("magic_data.rs");
    let mut f = File::create(&dest_path).unwrap();

    let mut magic_table = Vec::with_capacity(TABLE_CAPACITY);
    let mut current_offset = 0usize;
    let mut rng = Prng::new();

    let mut rook_magics = [SMagic::default(); 64];
    let mut bishop_magics = [SMagic::default(); 64];

    for_each_square!(sq => {
        rook_magics[sq as usize] =
            find_magic(Slider::Rook, sq, &mut current_offset, &mut magic_table, &mut rng);
    });

    for_each_square!(sq => {
        bishop_magics[sq as usize] =
            find_magic(Slider::Bishop, sq, &mut current_offset, &mut magic_table, &mut rng);
    });

    writeln!(
        f,
        r#"#[derive(Clone, Copy, Debug)]
pub struct SMagic {{
    pub mask: u64,
    pub magic: u64,
    pub offset: u32,
    pub shift: u8,
}}

pub static MAGIC_TABLE: [u64; {table_len}] = {magic_table:?};
pub const ROOK_MAGICS: [SMagic; 64] = {rook_magics:?};
pub const BISHOP_MAGICS: [SMagic; 64] = {bishop_magics:?};
"#,
        table_len = magic_table.len(),
        magic_table = magic_table,
        rook_magics = rook_magics,
        bishop_magics = bishop_magics,
    )
    .unwrap();
}

fn generate_nnue_data(out_dir: &str) {
    let nnue_path = Path::new("src/nnue/lucky-v7.bin");
    println!("cargo:rerun-if-changed={}", nnue_path.display());
    let bytes = std::fs::read(nnue_path).expect("Failed to read NNUE file");

    let rs_path = Path::new(out_dir).join("nnue_data.rs");

    // Universal builds link several copies of the engine into one executable,
    // so the launcher embeds the network once and every copy refers to it
    if env::var_os("CARGO_CFG_LUCKY_EXTERNAL_NETWORK").is_some() {
        let code = format!(
            r#"unsafe extern "C" {{
    #[link_name = "lucky_nnue_network"]
    static EXTERNAL_NETWORK: Network;
}}

const _: () = assert!({len} == core::mem::size_of::<Network>());

pub static NNUE: &Network = unsafe {{ &EXTERNAL_NETWORK }};
"#,
            len = bytes.len()
        );
        std::fs::write(&rs_path, code).expect("Failed to write nnue_data.rs");
        return;
    }

    let bin_path = Path::new(out_dir).join("network.bin");
    std::fs::write(&bin_path, &bytes).expect("Failed to write network.bin");

    let align = match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => 2 * 1024 * 1024,
        _ => 64,
    };

    let code = format!(
        r#"#[repr(C, align({align}))]
struct AlignedData([u8; {len}]);

static RAW_DATA: AlignedData = AlignedData(*include_bytes!("network.bin"));

const _: () = assert!({len} == core::mem::size_of::<Network>());

pub static NNUE: &Network = unsafe {{
    &*(RAW_DATA.0.as_ptr() as *const Network)
}};
"#,
        len = bytes.len()
    );
    std::fs::write(&rs_path, code).expect("Failed to write nnue_data.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    generate_pext_data(&out_dir);
    generate_magic_data(&out_dir);
    generate_nnue_data(&out_dir);
}
