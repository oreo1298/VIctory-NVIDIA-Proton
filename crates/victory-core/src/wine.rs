//! Whether this Windows program runs under Wine (Proton, umu-launcher, plain
//! Wine), and what Wine says of itself. Every behaviour that must differ under
//! Wine asks [`is_wine`], so one switch turns all of it on or off:
//!
//! `VICTORY_PROTON=1`: behave as under Wine whatever the detection says;
//! `VICTORY_PROTON=0`: behave as on Windows even under Wine; unset (or any
//! other value): detect. Detection is Wine's own ntdll export
//! `wine_get_version`, which Windows' ntdll does not have. The override is
//! for testing either path; under real Wine, `VICTORY_PROTON=0` brings back
//! the Windows paths with their costs there (docs/proton-audit.md).
//!
//! Nothing here runs title code or changes anything; it only reads.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What Wine says of itself ([`host`]).
#[derive(Debug, Clone, Default)]
pub struct WineHost {
    /// `wine_get_version`, e.g. `10.0`.
    pub version: String,
    /// `wine_get_build_id`, e.g. `wine-10.0 (Proton-10.0-3)`, where exported.
    pub build: Option<String>,
    /// `wine_get_host_version`: the Unix system and its release, e.g.
    /// `Linux` and `6.17.8-2-cachyos`.
    pub system: Option<String>,
    pub release: Option<String>,
}

/// Wine's own description of itself, if this process runs under Wine
/// (whatever `VICTORY_PROTON` says): looked up once.
pub fn host() -> Option<&'static WineHost> {
    static HOST: OnceLock<Option<WineHost>> = OnceLock::new();
    HOST.get_or_init(detect).as_ref()
}

/// Whether Proton-specific behaviour is on: `VICTORY_PROTON` if it is `0` or
/// `1` (read once), else whether Wine was detected ([`host`]). Always false
/// in a build for another OS than Windows (the tests' host).
pub fn is_wine() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(
        || match std::env::var("VICTORY_PROTON").as_deref().map(str::trim) {
            Ok("1") => cfg!(windows),
            Ok("0") => false,
            _ => host().is_some(),
        },
    )
}

/// One line for the log saying what was detected and the environment that
/// matters under Proton (NVAPI, the Proton build the launcher used); `None`
/// on Windows. (Logged by `victory-gpu`'s `proton` module: this crate has no
/// logger of its own.)
pub fn describe() -> Option<String> {
    let host = host()?;
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    Some(format!(
        "Wine {} (build {}) on {} {}; Proton {}; PROTON_ENABLE_NVAPI={} DXVK_ENABLE_NVAPI={}; Proton paths {} (VICTORY_PROTON=0|1 overrides)",
        host.version,
        host.build.as_deref().unwrap_or("-"),
        host.system.as_deref().unwrap_or("-"),
        host.release.as_deref().unwrap_or("-"),
        env("PROTONPATH")
            .or_else(|| env("STEAM_COMPAT_TOOL_PATHS"))
            .as_deref()
            .unwrap_or("-"),
        env("PROTON_ENABLE_NVAPI").as_deref().unwrap_or("unset"),
        env("DXVK_ENABLE_NVAPI").as_deref().unwrap_or("unset"),
        if is_wine() { "on" } else { "off" },
    ))
}

/// `path` as Windows code under Wine reads it: an absolute Unix path
/// (`/home/me/x`) becomes `Z:\home\me\x`, since Wine and Proton map `Z:` to
/// `/`. Anything else, and everything when not [`is_wine`], is returned as
/// it is. (A bare Unix path means "the root of the current drive" to Windows
/// code, which is right only while the current folder is on `Z:`.)
pub fn windows_path(path: &Path) -> PathBuf {
    if !is_wine() {
        return path.to_path_buf();
    }
    match path.to_str().and_then(unix_to_dos) {
        Some(dos) => PathBuf::from(dos),
        None => path.to_path_buf(),
    }
}

/// `Z:` + the path with `\` for `/`, for an absolute Unix path (one `/`
/// first, not two: `//server/share` is a Windows UNC name).
pub fn unix_to_dos(path: &str) -> Option<String> {
    (path.starts_with('/') && !path.starts_with("//"))
        .then(|| format!("Z:{}", path.replace('/', "\\")))
}

#[cfg(windows)]
fn detect() -> Option<WineHost> {
    use std::ffi::{CStr, c_char, c_void};
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    type Text = unsafe extern "C" fn() -> *const c_char;
    type HostVersion = unsafe extern "C" fn(*mut *const c_char, *mut *const c_char);
    let text = |p: *const c_char| {
        // SAFETY: Wine returns static NUL-terminated strings, or null.
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    };
    // SAFETY: plain lookups in ntdll, which every process has loaded; the
    // exports' signatures are Wine's (`include/wine/library.h`, ntdll.spec).
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr());
        if ntdll.is_null() {
            return None;
        }
        let version = GetProcAddress(ntdll, c"wine_get_version".as_ptr());
        if version.is_null() {
            return None;
        }
        let version: Text = std::mem::transmute(version);
        let build = GetProcAddress(ntdll, c"wine_get_build_id".as_ptr());
        let build = (!build.is_null()).then(|| std::mem::transmute::<*mut c_void, Text>(build));
        let mut host = WineHost {
            version: text(version()).unwrap_or_default(),
            build: build.and_then(|f| text(f())),
            ..Default::default()
        };
        let host_version = GetProcAddress(ntdll, c"wine_get_host_version".as_ptr());
        if !host_version.is_null() {
            let host_version: HostVersion = std::mem::transmute(host_version);
            let (mut system, mut release) = (std::ptr::null(), std::ptr::null());
            host_version(&mut system, &mut release);
            host.system = text(system);
            host.release = text(release);
        }
        Some(host)
    }
}

#[cfg(not(windows))]
fn detect() -> Option<WineHost> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_paths_become_drive_z() {
        assert_eq!(
            unix_to_dos("/home/me/Games/dump").as_deref(),
            Some(r"Z:\home\me\Games\dump")
        );
        assert_eq!(unix_to_dos("/").as_deref(), Some(r"Z:\"));
        // Windows paths, UNC names and relative paths are left alone.
        assert_eq!(unix_to_dos(r"D:\Games\dump"), None);
        assert_eq!(unix_to_dos("//server/share"), None);
        assert_eq!(unix_to_dos("relative/dir"), None);
        assert_eq!(unix_to_dos(r"Z:\already"), None);
    }

    #[test]
    fn nothing_changes_off_wine() {
        // The tests run on the build host, never under Wine.
        if host().is_none() && std::env::var_os("VICTORY_PROTON").is_none() {
            assert!(!is_wine());
            assert_eq!(
                windows_path(Path::new("/home/me")),
                PathBuf::from("/home/me")
            );
        }
    }
}
