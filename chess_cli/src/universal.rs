//! See `scripts/build_universal.sh` for how the copies are built and linked in.

use std::arch::is_x86_feature_detected as has;
use std::ffi::{CString, c_char, c_int};
use std::os::unix::ffi::OsStringExt;

type Entry = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

unsafe extern "C" {
    fn lucky_main_x86_64_v2(argc: c_int, argv: *const *const c_char) -> c_int;
    fn lucky_main_x86_64_v3_nopext(argc: c_int, argv: *const *const c_char) -> c_int;
    fn lucky_main_x86_64_v3(argc: c_int, argv: *const *const c_char) -> c_int;
    fn lucky_main_x86_64_v4(argc: c_int, argv: *const *const c_char) -> c_int;
}

/// Must match SLICES in scripts/build_universal.sh
const SLICES: [(&str, Entry); 4] = [
    ("x86_64_v2", lucky_main_x86_64_v2),
    ("x86_64_v3_nopext", lucky_main_x86_64_v3_nopext),
    ("x86_64_v3", lucky_main_x86_64_v3),
    ("x86_64_v4", lucky_main_x86_64_v4),
];

macro_rules! network_bytes {
    () => {
        include_bytes!("../../chess_engine/src/nnue/lucky-v6.bin")
    };
}

/// 2 MB, for huge pages
#[repr(C, align(2097152))]
struct Aligned<T>(T);

/// Embedded once and shared by every engine copy, see `generate_nnue_data` in chess_engine/build.rs
#[unsafe(export_name = "lucky_nnue_network")]
static NETWORK: Aligned<[u8; network_bytes!().len()]> = Aligned(*network_bytes!());

/// AMD Excavator (family 15h), Zen 1/2 (family 17h) and Hygon Dhyana (Zen 1, family 18h)
fn has_slow_pext() -> bool {
    use std::arch::x86_64::__cpuid;

    let vendor = __cpuid(0);
    let vendor = [vendor.ebx, vendor.edx, vendor.ecx].map(u32::to_le_bytes);
    let eax = __cpuid(1).eax;
    let family = match (eax >> 8) & 0xF {
        0xF => 0xF + ((eax >> 20) & 0xFF),
        base => base,
    };

    match vendor.as_flattened() {
        b"AuthenticAMD" => matches!(family, 0x15 | 0x17),
        b"HygonGenuine" => family == 0x18,
        _ => false,
    }
}

fn select_slice() -> Option<&'static str> {
    let v2 = has!("cmpxchg16b") && has!("popcnt") && has!("sse3") && has!("ssse3")
        && has!("sse4.1") && has!("sse4.2");
    let v3 = v2 && has!("avx") && has!("avx2") && has!("bmi1") && has!("bmi2") && has!("f16c")
        && has!("fma") && has!("lzcnt") && has!("movbe") && has!("xsave");
    let v4 = v3 && has!("avx512f") && has!("avx512bw") && has!("avx512cd") && has!("avx512dq")
        && has!("avx512vl");

    match () {
        _ if v3 && has_slow_pext() => Some("x86_64_v3_nopext"),
        _ if v4 => Some("x86_64_v4"),
        _ if v3 => Some("x86_64_v3"),
        _ if v2 => Some("x86_64_v2"),
        _ => None,
    }
}

pub fn main() {
    let name = match std::env::var("LUCKY_CHESS_ARCH") {
        Ok(name) if name != "auto" => Some(name),
        _ => select_slice().map(str::to_owned),
    };
    let Some(name) = name else {
        eprintln!("This CPU does not support x86-64-v2 (SSE4.2 and POPCNT)");
        std::process::exit(1);
    };
    let Some(&(_, entry)) = SLICES.iter().find(|(n, _)| *n == name) else {
        let names: Vec<_> = SLICES.iter().map(|(n, _)| *n).collect();
        eprintln!("Unknown build '{name}', available: auto, {}", names.join(", "));
        std::process::exit(1);
    };

    let args: Vec<CString> = std::env::args_os()
        .map(|arg| CString::new(arg.into_vec()).expect("argument contains a NUL byte"))
        .collect();
    let argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();

    // SAFETY: argv holds args.len() valid C strings that outlive the call
    let code = unsafe { entry(argv.len() as c_int, argv.as_ptr()) };
    std::process::exit(code);
}
