//! This process's memory, for the measurements in `loupe_platform::metrics`.

use std::mem::size_of;

use windows::Win32::System::{
    ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
    Threading::GetCurrentProcess,
};

/// `(working set, committed private)` in bytes: the physical memory mapped
/// into the process (Task Manager's details tab) and its private commit
/// (close to Task Manager's "Memory" column). `None` if Windows won't say.
pub fn memory() -> Option<(u64, u64)> {
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    ok.as_bool().then_some((
        counters.WorkingSetSize as u64,
        counters.PagefileUsage as u64,
    ))
}
