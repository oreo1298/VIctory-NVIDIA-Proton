//! What a run under Wine (Proton) says about itself, beside the Windows run's
//! lines: what Wine it is, which kind of write watching Wine gives (the cost
//! of noticing the title's writes, docs/proton-audit.md L9), and how close the
//! process comes to Linux's limit on memory mappings (L11). Nothing here runs
//! on Windows, or under Wine with `VICTORY_PROTON=0`; nothing here changes
//! what the layer does.

use std::time::{Duration, Instant};

/// Logs, once, what Wine says of itself and measures its write watching.
/// Under Wine only; cheap (about 16 MiB touched twice).
pub fn log_start() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(line) = victory_core::wine::describe() else {
            return;
        };
        tracing::info!("{line}");
        if victory_core::wine::is_wine() {
            write_watch_test();
        }
    });
}

/// Writes a page of each of 4,096 pages of write-watched memory after the
/// watch was reset, and a second time without: the difference is what a
/// first write costs. Wine gives write watches in the kernel (userfaultfd
/// with asynchronous write protection and `PAGEMAP_SCAN`, Linux 6.7 and a
/// Wine built with them): a minor fault in the kernel, well under 2 us. Or it
/// write-protects the pages and takes a signal for each first write: several
/// us each, under one lock for the whole process, and a split mapping each.
/// Also checks that the watch named every page written.
fn write_watch_test() {
    use windows_sys::Win32::System::Memory::{
        GetWriteWatch, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, ResetWriteWatch,
        VirtualAlloc, VirtualFree,
    };
    const MEM_WRITE_WATCH: u32 = 0x20_0000;
    const PAGES: usize = 4096;
    const PAGE: usize = 4096;
    let size = PAGES * PAGE;
    // SAFETY: a fresh allocation of ours, written only inside, freed at the end.
    unsafe {
        let at = VirtualAlloc(
            std::ptr::null(),
            size,
            MEM_RESERVE | MEM_COMMIT | MEM_WRITE_WATCH,
            PAGE_READWRITE,
        )
        .cast::<u8>();
        if at.is_null() {
            tracing::warn!(error = %std::io::Error::last_os_error(), "write watch test: no write-watched memory");
            return;
        }
        let touch = |value: u8| {
            let began = Instant::now();
            for page in 0..PAGES {
                at.add(page * PAGE).write_volatile(value);
            }
            began.elapsed()
        };
        touch(1); // the pages are there now
        ResetWriteWatch(at.cast(), size);
        let watched = touch(2);
        let plain = touch(3);
        let mut found = vec![std::ptr::null_mut(); PAGES + 1];
        let (mut count, mut granularity) = (found.len(), 0u32);
        let asked = GetWriteWatch(
            0,
            at.cast(),
            size,
            found.as_mut_ptr(),
            &mut count,
            &mut granularity,
        );
        VirtualFree(at.cast(), 0, MEM_RELEASE);
        let per_page = watched.saturating_sub(plain).as_secs_f64() * 1e6 / PAGES as f64;
        let kernel = per_page < 1.5;
        tracing::info!(
            first_write_us = format!("{per_page:.2}"),
            pages = PAGES,
            noticed = if asked == 0 { count } else { 0 },
            "Wine write watches: {}",
            if kernel {
                "in the kernel (userfaultfd), cheap"
            } else {
                "by write protection and a signal per first write"
            }
        );
        if !kernel {
            tracing::warn!(
                "every first write of the title to its GPU memory after a copy costs a signal here ({per_page:.1} us; \
                 several thousand a frame). Kernel write watches make them cheap: Linux 6.7 or later and a Wine \
                 that has them (upstream since 10.7, commit c52eab90; current proton-cachyos and GE-Proton builds \
                 are expected to)"
            );
        }
        if asked != 0 || count < PAGES {
            tracing::warn!(
                noticed = count,
                of = PAGES,
                "write watches did not name every page written: the GPU may read stale title data"
            );
        }
    }
}

/// Under Wine, starts a thread that looks at the process's memory mappings
/// every 10 s and warns once past half and past four fifths of
/// `vm.max_map_count`: past it Linux refuses new mappings and protection
/// changes, and title memory is then not mapped, or a write to it not
/// noticed. (Each view of the title's memory is a mapping of its own, and so
/// is each guarded block inside one.)
pub fn watch_map_count() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    if !victory_core::wine::is_wine() {
        return;
    }
    ONCE.call_once(|| {
        let _ = std::thread::Builder::new().name("victory map count".into()).spawn(|| {
            use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_LOWEST};
            // SAFETY: this thread's own priority.
            unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST) };
            let Some(limit) = std::fs::read_to_string(r"Z:\proc\sys\vm\max_map_count")
                .ok()
                .and_then(|t| t.trim().parse::<u64>().ok())
            else {
                tracing::info!("no vm.max_map_count to watch (Z:\\proc not readable)");
                return;
            };
            let mut said = 0u64;
            let mut first = true;
            loop {
                let Some(count) = mappings() else { return };
                if first {
                    tracing::info!(mappings = count, limit, "memory mappings of the process (vm.max_map_count)");
                    first = false;
                }
                for (share, what) in [(50, "half"), (80, "four fifths")] {
                    if count * 100 >= limit * share && said < share {
                        said = share;
                        tracing::warn!(
                            mappings = count,
                            limit,
                            "the process uses {what} of vm.max_map_count; past it mappings fail. Raise it: \
                             sudo sysctl -w vm.max_map_count=2147483642 (and in /etc/sysctl.d/ to keep it)"
                        );
                    }
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        });
    });
}

/// How many mappings the process has: the lines of `/proc/self/maps`, read
/// in pieces without keeping them.
fn mappings() -> Option<u64> {
    use std::io::Read;
    let mut file = std::fs::File::open(r"Z:\proc\self\maps").ok()?;
    let mut buffer = vec![0u8; 1 << 16];
    let mut lines = 0u64;
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => lines += buffer[..n].iter().filter(|&&b| b == b'\n').count() as u64,
            Err(_) => return None,
        }
    }
    Some(lines)
}

#[cfg(test)]
mod tests {
    /// Under Wine, the process's mappings are read through `Z:\proc` (on
    /// Windows there is nothing to read, and the test says so).
    #[test]
    fn mappings_are_counted_under_wine() {
        if !victory_core::wine::is_wine() {
            assert!(super::mappings().is_none());
            return;
        }
        assert!(super::mappings().is_some_and(|n| n > 10));
    }
}
