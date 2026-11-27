//! Which vendor's card the host device is on, for the few places that differ
//! by vendor (`crate::indirect::amd`: AMD cards run indirect dispatches wider
//! than D3D12's 65,535 groups).
//!
//! On Windows this is DXGI's vendor ID, as before. Under Wine, DXGI is DXVK's,
//! and DXVK reports an NVIDIA card as an AMD one (vendor 0x1002, device
//! 0x73df, the description unchanged) unless NVAPI is enabled
//! (`dxgi.hideNvidiaGpu = Auto`, DXVK's `dxgi_adapter.cpp`). There the real
//! identity is taken from the Vulkan device behind DXGI's adapter
//! (`IDXGIVkInteropAdapter::GetVulkanHandles`, then
//! `vkGetPhysicalDeviceProperties`), and, where that cannot be asked, an
//! "NVIDIA ..." description under AMD's ID is taken for NVIDIA.
//!
//! `VICTORY_GPU_VENDOR=auto|nvidia|amd|intel|other`: `auto` (or unset) as
//! above; any other value is used as the vendor whatever the card says
//! (to try a vendor's path, or to correct a wrong detection).

use std::sync::Mutex;

use windows::Win32::Graphics::Direct3D12::ID3D12Device;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIAdapter1, IDXGIFactory4,
};
use windows::core::Interface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

impl Vendor {
    pub fn from_id(id: u32) -> Vendor {
        match id {
            0x10de => Vendor::Nvidia,
            0x1002 | 0x1022 => Vendor::Amd,
            0x8086 => Vendor::Intel,
            _ => Vendor::Other,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Vendor::Nvidia => "NVIDIA",
            Vendor::Amd => "AMD",
            Vendor::Intel => "Intel",
            Vendor::Other => "other",
        }
    }
}

/// `VICTORY_GPU_VENDOR` (see the module's notes); `None` for auto.
fn forced() -> Option<Vendor> {
    match std::env::var("VICTORY_GPU_VENDOR")
        .ok()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "nvidia" => Some(Vendor::Nvidia),
        "amd" => Some(Vendor::Amd),
        "intel" => Some(Vendor::Intel),
        "other" => Some(Vendor::Other),
        _ => None,
    }
}

/// The vendor of the card `device` is on (looked up once per adapter).
pub(crate) fn of(device: &ID3D12Device) -> Vendor {
    static SEEN: Mutex<Vec<(i64, Vendor)>> = Mutex::new(Vec::new());
    // SAFETY: a plain query of a live device.
    let luid = unsafe { device.GetAdapterLuid() };
    let key = (i64::from(luid.HighPart) << 32) | i64::from(luid.LowPart);
    if let Some(&(_, v)) = SEEN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(k, _)| *k == key)
    {
        return v;
    }
    let vendor = identify(luid);
    SEEN.lock()
        .unwrap_or_else(|e| e.into_inner())
        .push((key, vendor));
    vendor
}

fn identify(luid: windows::Win32::Foundation::LUID) -> Vendor {
    // SAFETY: plain DXGI queries.
    let adapter = unsafe {
        CreateDXGIFactory2::<IDXGIFactory4>(DXGI_CREATE_FACTORY_FLAGS(0))
            .and_then(|f| f.EnumAdapterByLuid::<IDXGIAdapter1>(luid))
            .ok()
    };
    // SAFETY: a plain query of a live adapter.
    let desc = adapter.as_ref().and_then(|a| unsafe { a.GetDesc1() }.ok());
    let (dxgi_id, description) = desc.map_or((0, String::new()), |d| {
        let len = d
            .Description
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(d.Description.len());
        (d.VendorId, String::from_utf16_lossy(&d.Description[..len]))
    });
    let from_dxgi = Vendor::from_id(dxgi_id);
    if let Some(v) = forced() {
        tracing::info!(vendor = v.name(), dxgi = format_args!("{dxgi_id:#x}"), card = %description, "GPU vendor set by VICTORY_GPU_VENDOR");
        return v;
    }
    if !victory_core::wine::is_wine() {
        return from_dxgi;
    }
    let vulkan = adapter.as_ref().and_then(vulkan_identity);
    let (vendor, how) = match &vulkan {
        Some((id, _)) => (Vendor::from_id(*id), "the Vulkan device"),
        None if from_dxgi == Vendor::Amd && description.to_ascii_uppercase().contains("NVIDIA") => {
            (
                Vendor::Nvidia,
                "the adapter's description (DXVK hides NVIDIA's ID)",
            )
        }
        None => (from_dxgi, "DXGI"),
    };
    tracing::info!(
        vendor = vendor.name(),
        by = how,
        dxgi = format_args!("{dxgi_id:#x}"),
        vulkan = vulkan.as_ref().map_or("-".to_owned(), |(id, name)| format!("{id:#x} {name}")),
        card = %description,
        "GPU vendor (VICTORY_GPU_VENDOR=nvidia|amd|... overrides)"
    );
    if vendor != from_dxgi {
        tracing::warn!(
            "DXGI reports this {} card as {} (DXVK does so without NVAPI): the layer uses the card's own vendor; \
             start with PROTON_ENABLE_NVAPI=1 and DXVK_ENABLE_NVAPI=1 (scripts/play-nvidia.sh does) for DLSS and Reflex",
            vendor.name(),
            from_dxgi.name()
        );
    }
    vendor
}

/// The Vulkan vendor ID and device name behind a DXVK adapter, where it is
/// one (Wine with DXVK's DXGI): `IDXGIVkInteropAdapter`, DXVK's
/// `dxgi_interfaces.h`.
fn vulkan_identity(adapter: &IDXGIAdapter1) -> Option<(u32, String)> {
    use std::ffi::{c_char, c_void};
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    const INTEROP: windows::core::GUID =
        windows::core::GUID::from_u128(0x3a6d8f2c_b0e8_4ab4_b4dc_4fd24891bfa5);
    type Handles = unsafe extern "system" fn(*mut c_void, *mut usize, *mut usize);
    type Release = unsafe extern "system" fn(*mut c_void) -> u32;
    type InstanceProc = unsafe extern "system" fn(usize, *const c_char) -> *mut c_void;
    type Properties = unsafe extern "system" fn(usize, *mut u8);
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: COM's own query; `raw` holds a reference on success.
    unsafe { adapter.query(&INTEROP, &mut raw) }.ok().ok()?;
    if raw.is_null() {
        return None;
    }
    let (mut instance, mut physical) = (0usize, 0usize);
    // SAFETY: the interface's vtable: IUnknown's three, then GetVulkanHandles;
    // the reference the query took is given back.
    unsafe {
        let vtable = *(raw as *const *const usize);
        let handles: Handles = std::mem::transmute(*vtable.add(3));
        handles(raw, &mut instance, &mut physical);
        let release: Release = std::mem::transmute(*vtable.add(2));
        release(raw);
    }
    if instance == 0 || physical == 0 {
        return None;
    }
    // The loader DXVK took its functions from (winevulkan first, as DXVK).
    let module = ["winevulkan.dll", "vulkan-1.dll"].iter().find_map(|name| {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated name; only a loaded module is taken.
        let m = unsafe { GetModuleHandleW(wide.as_ptr()) };
        (!m.is_null()).then_some(m)
    })?;
    // SAFETY: Vulkan's entry points by their names, with Vulkan's signatures;
    // the properties are written into a zeroed buffer larger than
    // VkPhysicalDeviceProperties (824 bytes): vendorID at 8, deviceName
    // (256 bytes) at 20.
    unsafe {
        let get = GetProcAddress(module, c"vkGetInstanceProcAddr".as_ptr().cast())?;
        let get: InstanceProc = std::mem::transmute(get);
        let properties = get(instance, c"vkGetPhysicalDeviceProperties".as_ptr());
        if properties.is_null() {
            return None;
        }
        let properties: Properties = std::mem::transmute(properties);
        let mut buffer = vec![0u64; 256];
        let bytes = buffer.as_mut_ptr().cast::<u8>();
        properties(physical, bytes);
        let all = std::slice::from_raw_parts(bytes, 276);
        let vendor = u32::from_le_bytes([all[8], all[9], all[10], all[11]]);
        let name = &all[20..276];
        let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        Some((vendor, String::from_utf8_lossy(&name[..len]).into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_ids() {
        assert_eq!(Vendor::from_id(0x10de), Vendor::Nvidia);
        assert_eq!(Vendor::from_id(0x1002), Vendor::Amd);
        assert_eq!(Vendor::from_id(0x8086), Vendor::Intel);
        assert_eq!(Vendor::from_id(0x1414), Vendor::Other);
    }
}
