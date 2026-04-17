use akatsuki_pp_rs::{
    any::PerformanceAttributes,
    model::mode::GameMode,
    osu_2019::{stars::OsuPerformanceAttributes, OsuPP},
    Beatmap,
};
use interoptopus::{
    extra_type, ffi_function, ffi_type, function,
    patterns::{option::FFIOption, slice::FFISlice},
    Inventory, InventoryBuilder,
};
use std::ffi::CStr;
use std::os::raw::c_char;

#[ffi_type]
#[repr(C)]
#[derive(Clone, Default, PartialEq)]
pub struct CalculatePerformanceResult {
    pub pp: f64,
    pub stars: f64,
    pub ar: f64,
    pub od: f64,
    pub max_combo: u32,
}

impl std::fmt::Display for CalculatePerformanceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("CalculatePerformanceResult");
        s.field("pp", &self.pp)
            .field("stars", &self.stars)
            .field("ar", &self.ar)
            .field("od", &self.od)
            .field("max_combo", &self.max_combo);

        s.finish()
    }
}

impl CalculatePerformanceResult {
    fn from_attributes(attributes: PerformanceAttributes) -> Self {
        Self {
            pp: attributes.pp(),
            stars: attributes.stars(),
            ar: match attributes {
                PerformanceAttributes::Osu(ref attrs) => attrs.difficulty.ar,
                _ => 0.0,
            },
            od: match attributes {
                PerformanceAttributes::Osu(ref attrs) => attrs.difficulty.od(),
                _ => 0.0,
            },
            max_combo: attributes.max_combo(),
        }
    }

    fn from_rx_attributes(attributes: OsuPerformanceAttributes) -> Self {
        Self {
            pp: attributes.pp,
            stars: attributes.difficulty.stars,
            ar: attributes.difficulty.ar,
            od: attributes.difficulty.od,
            max_combo: attributes.difficulty.max_combo as u32,
        }
    }
}

fn calculate_performance(
    beatmap: Beatmap,
    mode: u32,
    mods: u32,
    max_combo: u32,
    accuracy: Option<f64>,
    count_300: Option<u32>,
    count_100: Option<u32>,
    count_50: Option<u32>,
    miss_count: u32,
    passed_objects: Option<u32>,
) -> CalculatePerformanceResult {
    // osu!std rx

    if mode == 0 && mods & 128 > 0 {
        let mut calculator = OsuPP::from_map(&beatmap);
        calculator = calculator.mods(mods).combo(max_combo).misses(miss_count);

        if let Some(passed_objects) = passed_objects {
            calculator = calculator.passed_objects(passed_objects);
        }

        if let Some(accuracy) = accuracy {
            calculator = calculator.accuracy(accuracy as f32);
        } else {
            calculator = calculator
                .n300(count_300.unwrap())
                .n100(count_100.unwrap())
                .n50(count_50.unwrap());
        }

        let rosu_result = calculator.calculate();
        CalculatePerformanceResult::from_rx_attributes(rosu_result)
    } else {
        let mut calculator = beatmap
            .performance()
            .try_mode(match mode {
                0 => GameMode::Osu,
                1 => GameMode::Taiko,
                2 => GameMode::Catch,
                3 => GameMode::Mania,
                _ => panic!("Invalid mode"),
            })
            .unwrap()
            .mods(mods)
            .lazer(false)
            .combo(max_combo)
            .misses(miss_count);

        if let Some(passed_objects) = passed_objects {
            calculator = calculator.passed_objects(passed_objects);
        }

        if let Some(accuracy) = accuracy {
            calculator = calculator.accuracy(accuracy);
        } else {
            calculator = calculator
                .n300(count_300.unwrap())
                .n100(count_100.unwrap())
                .n50(count_50.unwrap());
        }

        let rosu_result = calculator.calculate();
        CalculatePerformanceResult::from_attributes(rosu_result)
    }
}

#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn calculate_performance_from_path(
    beatmap_path: *const c_char,
    mode: u32,
    mods: u32,
    max_combo: u32,
    accuracy: FFIOption<f64>,
    count_300: FFIOption<u32>,
    count_100: FFIOption<u32>,
    count_50: FFIOption<u32>,
    miss_count: u32,
    passed_objects: FFIOption<u32>,
) -> CalculatePerformanceResult {
    let beatmap = Beatmap::from_path(CStr::from_ptr(beatmap_path).to_str().unwrap()).unwrap();

    calculate_performance(
        beatmap,
        mode,
        mods,
        max_combo,
        accuracy.into_option(),
        count_300.into_option(),
        count_100.into_option(),
        count_50.into_option(),
        miss_count,
        passed_objects.into_option(),
    )
}

// This is bad. I know.
static mut CACHED_BEATMAP: Option<Beatmap> = None;
static mut CACHED_BEATMAP_LEN: usize = 0;

#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn calculate_performance_from_bytes(
    beatmap_bytes: FFISlice<u8>,
    mode: u32,
    mods: u32,
    max_combo: u32,
    accuracy: FFIOption<f64>,
    count_300: FFIOption<u32>,
    count_100: FFIOption<u32>,
    count_50: FFIOption<u32>,
    miss_count: u32,
    passed_objects: FFIOption<u32>,
) -> CalculatePerformanceResult {
    let beatmap: Beatmap;

    // FUCK that this is unsafe!
    // we need PURE PERFORMANCE!
    if CACHED_BEATMAP_LEN == beatmap_bytes.len() && CACHED_BEATMAP.is_some() {
        beatmap = CACHED_BEATMAP.clone().unwrap();
    } else {
        beatmap = Beatmap::from_bytes(beatmap_bytes.as_slice()).unwrap();
        CACHED_BEATMAP = Some(beatmap.clone());
        CACHED_BEATMAP_LEN = beatmap_bytes.len();
    }

    calculate_performance(
        beatmap,
        mode,
        mods,
        max_combo,
        accuracy.into_option(),
        count_300.into_option(),
        count_100.into_option(),
        count_50.into_option(),
        miss_count,
        passed_objects.into_option(),
    )
}

pub fn my_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(extra_type!(CalculatePerformanceResult))
        .register(function!(calculate_performance_from_path))
        .register(function!(calculate_performance_from_bytes))
        .inventory()
}

#[test]
fn generate_csharp_bindings() {
    use interoptopus::Interop;
    use interoptopus_backend_csharp::{Config, Generator};

    let inventory = my_inventory();

    let config = Config {
        dll_name: "akatsuki_pp".to_string(),
        ..Config::default()
    };

    Generator::new(config, inventory)
        .write_file("bindings/AkatsukiPPFFI.cs")
        .unwrap();
}

#[test]
fn generate_c_bindings() {
    use interoptopus::Interop;
    use interoptopus_backend_c::{Config, Generator};

    let inventory = my_inventory();

    let config = Config {
        ifndef: "akatsuki_pp".to_string(),
        ..Config::default()
    };

    Generator::new(config, inventory)
        .write_file("bindings/akatsuki_pp_ffi.h")
        .unwrap();
}

#[test]
fn generate_cpython_bindings() {
    use interoptopus::Interop;
    use interoptopus_backend_cpython::{Config, Generator};

    let inventory = my_inventory();

    let config = Config::default();

    Generator::new(config, inventory)
        .write_file("bindings/akatsuki_pp_ffi.py")
        .unwrap();
}
