//! GPU use as Task Manager shows it: Windows' "GPU Engine" counters, which give a percentage for
//! every engine (3D, copy, video decode, …) of every process.

use std::collections::HashMap;

use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{PCWSTR, w};

/// PDH's "the buffer was too small" answer (0x800007D2), which is how it reports the size needed.
const MORE_DATA: u32 = 0x8000_07D2;

/// Open only while the panel is, like the CPU counter.
pub struct Gpu {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    /// u64s so the items inside are aligned.
    buffer: Vec<u64>,
}

impl Gpu {
    /// None on a PC whose driver has no GPU counters (they came with Windows 10 1709).
    pub fn open() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }
        // Dropping it closes the query, even if the counter isn't there.
        let mut gpu = Self { query, counter: PDH_HCOUNTER::default(), buffer: Vec::new() };
        let path = w!(r"\GPU Engine(*)\Utilization Percentage");
        if unsafe { PdhAddEnglishCounterW(gpu.query, path, 0, &mut gpu.counter) } != 0 {
            return None;
        }
        unsafe { PdhCollectQueryData(gpu.query) }; // `read` reports use since now
        Some(gpu)
    }

    /// The busiest engine's use, in percent (0 to 100), since the previous reading.
    pub fn read(&mut self) -> Option<u8> {
        unsafe { PdhCollectQueryData(self.query) };
        let (mut size, mut count) = (0u32, 0u32);
        let status = unsafe { PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) };
        if status != MORE_DATA || count == 0 {
            return None;
        }
        self.buffer.resize((size as usize).div_ceil(8), 0);
        let items = self.buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let status = unsafe { PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items)) };
        if status != 0 {
            return None;
        }
        // SAFETY: PDH filled `count` items at the start of the buffer.
        let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
        busiest(items.iter().filter(|i| i.FmtValue.CStatus == 0).filter_map(|i| {
            let name = unsafe { i.szName.to_string() }.ok()?;
            Some((name, unsafe { i.FmtValue.Anonymous.doubleValue }))
        }))
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

/// Task Manager's number: add up each engine type's use across all processes (per GPU), and the
/// busiest of those is the GPU's use. Names look like
/// `pid_4242_luid_0x00000000_0x0000A1B2_phys_0_eng_0_engtype_3D`.
fn busiest(readings: impl Iterator<Item = (String, f64)>) -> Option<u8> {
    let mut sums: HashMap<(String, String), f64> = HashMap::new();
    for (name, value) in readings {
        let (Some(luid), Some(engine)) = (name.split("_phys").next().and_then(|s| s.split("luid_").nth(1)), name.rsplit("engtype_").next()) else {
            continue;
        };
        *sums.entry((luid.to_owned(), engine.to_owned())).or_default() += value;
    }
    sums.values().copied().reduce(f64::max).map(|v| v.clamp(0.0, 100.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(name: &str, v: f64) -> (String, f64) {
        (name.to_owned(), v)
    }

    #[test]
    fn busiest_engine_across_processes() {
        let readings = [
            r("pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 20.0),
            r("pid_2_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 15.6),
            r("pid_2_luid_0x0_0xA_phys_0_eng_1_engtype_Copy", 30.0),
            // Another GPU, busier than the first's 3D engine but not than its sum.
            r("pid_3_luid_0x0_0xB_phys_0_eng_0_engtype_3D", 30.0),
        ];
        assert_eq!(busiest(readings.into_iter()), Some(36), "3D on the first GPU: 20 + 15.6");
    }

    #[test]
    fn nothing_or_nonsense() {
        assert_eq!(busiest([].into_iter()), None);
        assert_eq!(busiest([r("_Total", 5.0)].into_iter()), None);
        assert_eq!(busiest([r("pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 250.0)].into_iter()), Some(100));
    }

    /// `cargo test gpu_cost -- --ignored --nocapture`: what reading the counters costs on this PC.
    #[test]
    #[ignore]
    fn gpu_cost() {
        let t = std::time::Instant::now();
        let mut gpu = Gpu::open().expect("no GPU counters");
        println!("open: {:?}", t.elapsed());
        for i in 0..8 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let t = std::time::Instant::now();
            let v = gpu.read();
            println!("read {i}: {:?} -> {v:?}", t.elapsed());
        }
    }
}
