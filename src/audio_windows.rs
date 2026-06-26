// SPDX-License-Identifier: MIT
// Windows audio backend using PowerShell + compiled C# helper.

#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use crate::audio::PactlDevice;
use crate::utils::append_log;

static HELPER_COMPILED: Mutex<bool> = Mutex::new(false);
static HELPER_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

// ── PowerShell runner ──

fn ps(script: &str) -> anyhow::Result<String> {
    let tmp = std::env::temp_dir().join("audio-selector-ps.ps1");
    std::fs::write(&tmp, script)?;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&tmp)
        .output()?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow::anyhow!("PowerShell: {}", err));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ── C# helper source ──

const CS_SOURCE: &str = r#"
using System;
using System.Runtime.InteropServices;

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E3"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {
    int EnumAudioEndpoints(int dataFlow, int dwStateMask, out object ppDevices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, out object ppDevice);
    int GetDevice([MarshalAs(UnmanagedType.LPWStr)] string pwstrId, out object ppDevice);
    int RegisterEndpointNotificationCallback(object pClient);
    int UnregisterEndpointNotificationCallback(object pClient);
}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice {
    int Activate(ref Guid riid, int dwClsCtx, IntPtr pActivationParams, [MarshalAs(UnmanagedType.IUnknown)] out object ppInterface);
    int OpenPropertyStore(int stgmAccess, out object ppProperties);
    int GetId([MarshalAs(UnmanagedType.LPWStr)] out string ppstrId);
    int GetState(out int pdwState);
}

[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {
    int RegisterControlChangeNotify(IntPtr pNotify);
    int UnregisterControlChangeNotify(IntPtr pNotify);
    int GetChannelCount(out int pnChannelCount);
    int SetMasterVolumeLevel(float fLevelDB, ref Guid pguidEventContext);
    int SetMasterVolumeLevelScalar(float fLevel, ref Guid pguidEventContext);
    int GetMasterVolumeLevel(out float pfLevelDB);
    int GetMasterVolumeLevelScalar(out float pfLevel);
    int SetChannelVolume(int nChannel, float fLevelDB, ref Guid pguidEventContext);
    int SetChannelVolumeScalar(int nChannel, float fLevel, ref Guid pguidEventContext);
    int GetChannelVolume(int nChannel, out float pfLevelDB);
    int GetChannelVolumeScalar(int nChannel, out float pfLevel);
    int SetMute(bool bMute, ref Guid pguidEventContext);
    int GetMute(out bool pbMute);
    int GetVolumeStepInfo(out int pnStep, out int pnStepCount);
    int VolumeStepUp(ref Guid pguidEventContext);
    int VolumeStepDown(ref Guid pguidEventContext);
    int QueryHardwareSupport(out int pdwHardwareSupportMask);
    int GetVolumeRange(out float pflMinVolumeDB, out float pflMaxVolumeDB, out float pflVolumeIncrementDB);
}

[Guid("F8679F50-850A-41CF-9C72-430F290290C8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig {
    int GetMixFormat(IntPtr, IntPtr);
    int GetDeviceFormat(IntPtr, bool, IntPtr);
    int ResetDeviceFormat(IntPtr);
    int SetDeviceFormat(IntPtr, IntPtr, IntPtr);
    int GetProcessingPeriod(IntPtr, bool, IntPtr);
    int SetProcessingPeriod(IntPtr, IntPtr);
    int GetShareMode(IntPtr, IntPtr);
    int SetShareMode(IntPtr, IntPtr);
    int GetPropertyValue(IntPtr, int, IntPtr, int, IntPtr);
    int SetPropertyValue(IntPtr, int, IntPtr, int, IntPtr);
    int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, int role);
    int SetEndpointVisibility(IntPtr, int);
}

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumeratorImpl { }

[ComImport, Guid("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9")]
class PolicyConfigImpl { }

class AudioHelper {
    static int Main(string[] args) {
        try {
            if (args.Length == 0) { Console.Error.WriteLine("no command"); return 1; }
            switch (args[0]) {
                case "set-default":
                    if (args.Length < 2) return 1;
                    SetDefault(args[1]);
                    return 0;
                case "volume-get":
                    if (args.Length < 2) return 1;
                    Console.WriteLine(GetVolume(args[1]).ToString("F6"));
                    return 0;
                case "volume-set":
                    if (args.Length < 3) return 1;
                    SetVolume(args[1], float.Parse(args[2]));
                    return 0;
                default:
                    Console.Error.WriteLine("unknown cmd: " + args[0]);
                    return 1;
            }
        } catch (Exception e) {
            Console.Error.WriteLine(e.Message);
            return 1;
        }
    }

    static void SetDefault(string deviceId) {
        var pc = (IPolicyConfig)new PolicyConfigImpl();
        for (int role = 0; role <= 2; role++)
            pc.SetDefaultEndpoint(deviceId, role);
    }

    static float GetVolume(string deviceId) {
        var e = (IMMDeviceEnumerator)new MMDeviceEnumeratorImpl();
        e.GetDevice(deviceId, out var devObj);
        var d = (IMMDevice)devObj;
        var iid = typeof(IAudioEndpointVolume).GUID;
        d.Activate(ref iid, 1, IntPtr.Zero, out var epvObj);
        var v = (IAudioEndpointVolume)epvObj;
        v.GetMasterVolumeLevelScalar(out var lvl);
        return lvl;
    }

    static void SetVolume(string deviceId, float level) {
        var e = (IMMDeviceEnumerator)new MMDeviceEnumeratorImpl();
        e.GetDevice(deviceId, out var devObj);
        var d = (IMMDevice)devObj;
        var iid = typeof(IAudioEndpointVolume).GUID;
        d.Activate(ref iid, 1, IntPtr.Zero, out var epvObj);
        var v = (IAudioEndpointVolume)epvObj;
        var g = Guid.Empty;
        v.SetMasterVolumeLevelScalar(level, ref g);
    }
}
"#;

// ── C# compiler detection & compilation ──

fn find_csc() -> Option<PathBuf> {
    let candidates = [
        r"C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe",
        r"C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe",
        r"C:\Windows\Microsoft.NET\Framework64\v3.5\csc.exe",
        r"C:\Windows\Microsoft.NET\Framework\v3.5\csc.exe",
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() { return Some(p); }
    }
    // Try PowerShell search
    if let Ok(out) = ps(&format!(
        r#"$p = Get-ChildItem "$env:windir\Microsoft.NET\Framework64\*\csc.exe" -ErrorAction SilentlyContinue | Sort Version -Descending | Select -First 1 -ExpandProperty FullName; if (-not $p) {{ $p = Get-ChildItem "$env:windir\Microsoft.NET\Framework\*\csc.exe" -ErrorAction SilentlyContinue | Sort Version -Descending | Select -First 1 -ExpandProperty FullName }}; Write-Output $p"#
    )) {
        let p = out.trim().to_string();
        if !p.is_empty() { return Some(PathBuf::from(p)); }
    }
    None
}

fn compile_helper() -> anyhow::Result<PathBuf> {
    let config_dir = crate::utils::get_config_dir();
    let _ = std::fs::create_dir_all(&config_dir);
    let cs_path = config_dir.join("audio-helper.cs");
    let exe_path = config_dir.join("audio-helper.exe");

    std::fs::write(&cs_path, CS_SOURCE)?;

    if let Some(csc) = find_csc() {
        let out = Command::new(&csc)
            .args(["/target:exe", "/nologo", "/optimize", "/debug-"])
            .arg(&format!("/out:{}", exe_path.display()))
            .arg(&cs_path)
            .output()?;
        if out.status.success() {
            append_log(&format!("C# helper compiled: {:?}", exe_path));
            return Ok(exe_path);
        }
        let err = String::from_utf8_lossy(&out.stderr);
        append_log(&format!("csc failed, fallback to Add-Type: {}", err));
    }

    // Fallback: compile via Add-Type in PowerShell
    let exe_path2 = config_dir.join("audio-helper2.exe");
    let ps_script = format!(
        r#"Add-Type -TypeDefinition @'{cs}'@ -OutputAssembly '{out}' -OutputType ConsoleApplication -ErrorAction Stop"#,
        cs = CS_SOURCE.replace("'", "''"),
        out = exe_path2.display().to_string().replace("'", "''")
    );
    let _ = ps(&ps_script)?;
    if exe_path2.exists() {
        std::fs::rename(&exe_path2, &exe_path)?;
    }
    append_log(&format!("C# helper compiled via Add-Type: {:?}", exe_path));
    Ok(exe_path)
}

fn ensure_helper() -> anyhow::Result<PathBuf> {
    let mut compiled = HELPER_COMPILED.lock().unwrap();
    if *compiled {
        return Ok(HELPER_PATH.lock().unwrap().clone().unwrap());
    }
    let exe = compile_helper()?;
    *HELPER_PATH.lock().unwrap() = Some(exe.clone());
    *compiled = true;
    Ok(exe)
}

fn run_helper(args: &[&str]) -> anyhow::Result<String> {
    let exe = ensure_helper()?;
    let out = Command::new(&exe).args(args).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow::anyhow!("helper failed: {}", err));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ── Public API ──

pub fn get_windows_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let flow = if target == "sources" { "1" } else { "0" };
    let script = format!(
        r#"$t = if ({flow} -eq 1) {{ "Capture" }} else {{ "Render" }}
$r = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\$t" -ErrorAction SilentlyContinue | ForEach-Object {{
    $id = $_.PSChildName
    $n = (Get-ItemProperty "$($_.PSPath)\Properties\{{a45c254e-df1c-4efd-8020-67d146a850e0}}\14" -ErrorAction SilentlyContinue)."(default)"
    if (-not $n) {{ $n = $id }}
    [PSCustomObject]@{{Name=$id; Description=$n}}
}}
if (-not $r) {{ $r = @() }}
ConvertTo-Json $r -Compress"#,
        flow = flow
    );
    let json = ps(&script)?;
    #[derive(serde::Deserialize)]
    struct PsDev { Name: String, Description: String }
    let devs: Vec<PsDev> = serde_json::from_str(&json).unwrap_or_default();
    Ok(devs.into_iter().map(|d| PactlDevice {
        name: d.Name,
        description: d.Description,
        volume: None,
    }).collect())
}

pub fn apply_windows_device_change(_target: &str, name: &str) -> anyhow::Result<()> {
    run_helper(&["set-default", name])?;
    append_log(&format!("Set default {}: {}", _target, name));
    Ok(())
}

pub fn set_windows_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    let level = (vol as f32).clamp(0.0, 100.0) / 100.0;
    run_helper(&["volume-set", name, &format!("{:.6}", level)])?;
    append_log(&format!("Set volume {}% for: {}", vol, name));
    Ok(())
}
