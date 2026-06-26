// SPDX-License-Identifier: MIT
// Windows audio backend using Core Audio API (WASAPI).

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Devices::Properties::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Com::StructuredStorage::*;

use crate::audio::PactlDevice;
use crate::utils::append_log;

// ── IPolicyConfig (undocumented COM interface for setting default endpoint) ──
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0xF8679F50_850A_41CF_9C72_430F290290C8);
const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xF8679F50_850A_41CF_9C72_430F290290C8);

#[repr(C)]
struct PolicyConfigVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    _m3: usize,
    _m4: usize,
    _m5: usize,
    set_default_endpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, u32) -> HRESULT,
    _rest: [usize; 10],
}

#[repr(transparent)]
struct PolicyConfig(*mut c_void);

impl PolicyConfig {
    unsafe fn create() -> anyhow::Result<Self> {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_POLICY_CONFIG,
            std::ptr::null(),
            CLSCTX_INPROC_SERVER,
            &IID_POLICY_CONFIG,
            &mut ptr,
        );
        if hr.is_err() || ptr.is_null() {
            return Err(anyhow::anyhow!("Failed to create PolicyConfig (COM)"));
        }
        Ok(PolicyConfig(ptr))
    }

    unsafe fn set_default(&self, device_id: PCWSTR, role: u32) -> anyhow::Result<()> {
        let vtbl = *(self.0 as *mut *const PolicyConfigVtbl);
        let func = (*vtbl).set_default_endpoint;
        let hr = func(self.0, device_id, role);
        if hr.is_err() {
            return Err(anyhow::anyhow!("SetDefaultEndpoint failed: {}", hr));
        }
        Ok(())
    }
}

impl Drop for PolicyConfig {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let vtbl = *(self.0 as *mut *const PolicyConfigVtbl);
                (*vtbl).release(self.0);
            }
        }
    }
}

// ── COM guard ──

struct ComGuard(bool);

impl ComGuard {
    fn new() -> anyhow::Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED)? };
        Ok(ComGuard(hr == S_OK))
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize(); }
        }
    }
}

// ── Helpers ──

fn create_enumerator() -> anyhow::Result<IMMDeviceEnumerator> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)? };
    Ok(enumerator)
}

fn friendly_name(store: &IPropertyStore) -> String {
    unsafe {
        match store.GetValue(&PKEY_Device_FriendlyName) {
            Ok(prop) => match prop.GetString() {
                Ok(bstr) => bstr.to_string().unwrap_or("Unknown".into()),
                Err(_) => "Unknown".into(),
            },
            Err(_) => "Unknown".into(),
        }
    }
}

fn device_volume(device: &IMMDevice) -> Option<i32> {
    unsafe {
        let ep_volume: IAudioEndpointVolume = match device.Activate(CLSCTX_INPROC_SERVER) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let mut level: f32 = 0.0;
        if ep_volume.GetMasterVolumeLevelScalar(&mut level).is_ok() {
            Some((level * 100.0).round() as i32)
        } else {
            None
        }
    }
}

fn flow(target: &str) -> EDataFlow {
    match target {
        "sources" => eCapture,
        _ => eRender,
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── Public API ──

pub fn get_windows_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let _com = ComGuard::new()?;
    let enumerator = create_enumerator()?;

    let collection = unsafe { enumerator.EnumAudioEndpoints(flow(target), DEVICE_STATE_ACTIVE)? };
    let count = unsafe { collection.GetCount()? };

    let mut devices = Vec::with_capacity(count as usize);
    for i in 0..count {
        let device = match unsafe { collection.Item(i) } {
            Ok(d) => d,
            Err(_) => continue,
        };

        let id = match unsafe { device.GetId() } {
            Ok(id) => format!("{}", id),
            Err(_) => continue,
        };

        let store = match unsafe { device.OpenPropertyStore(STGM_READ) } {
            Ok(s) => s,
            Err(_) => continue,
        };

        let name = friendly_name(&store);

        let volume = if target == "sinks" {
            device_volume(&device)
        } else {
            None
        };

        devices.push(PactlDevice {
            name: id,
            description: name,
            volume: volume.map(|v| {
                serde_json::json!({"0": {"value_percent": format!("{}%", v)}})
            }),
        });
    }

    Ok(devices)
}

pub fn apply_windows_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    let _com = ComGuard::new()?;
    let policy = unsafe { PolicyConfig::create()? };
    let w = wide(name);
    let id = PCWSTR::from_raw(w.as_ptr());

    unsafe {
        policy.set_default(id, 0)?;
        policy.set_default(id, 1)?;
        policy.set_default(id, 2)?;
    }

    append_log(&format!("Set default {}: {}", target, name));
    Ok(())
}

pub fn set_windows_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    let _com = ComGuard::new()?;
    let enumerator = create_enumerator()?;
    let w = wide(name);
    let id = PCWSTR::from_raw(w.as_ptr());

    let device = unsafe { enumerator.GetDevice(id)? };
    let ep_volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_INPROC_SERVER)? };

    let level = (vol as f32).clamp(0.0, 100.0) / 100.0;
    unsafe { ep_volume.SetMasterVolumeLevelScalar(level)?; }

    append_log(&format!("Set volume {}% for: {}", vol, name));
    Ok(())
}
