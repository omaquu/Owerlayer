#![allow(dead_code)]

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Media::Audio::*,
    Win32::Media::Audio::Endpoints::*,
    Win32::System::Com::*,
    Win32::System::Threading::*,
};

#[derive(Clone, Debug)]
pub struct AudioSessionInfo {
    pub name: String,
    pub pid: u32,
    pub volume: f32,
    pub mute: bool,
}

#[cfg(windows)]
unsafe fn get_process_name(pid: u32) -> String {
    let handle = match OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        false,
        pid,
    ) {
        Ok(h) => h,
        Err(_) => return String::new(),
    };
    
    let mut buffer = [0u8; 260];
    let mut size = buffer.len() as u32;
    if QueryFullProcessImageNameA(handle, PROCESS_NAME_FORMAT(0), windows::core::PSTR(buffer.as_mut_ptr()), &mut size).is_ok() {
        let path = String::from_utf8_lossy(&buffer[..size as usize]).into_owned();
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let _ = CloseHandle(handle);
        return name;
    }
    let _ = CloseHandle(handle);
    String::new()
}

#[cfg(windows)]
pub fn get_active_sessions() -> Vec<AudioSessionInfo> {
    let mut sessions = Vec::new();
    let master_vol = get_master_volume();
    let master_mute = get_master_mute();
    sessions.push(AudioSessionInfo {
        name: "Master".to_string(),
        pid: u32::MAX,
        volume: master_vol,
        mute: master_mute,
    });

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            Ok(e) => e,
            Err(_) => return sessions,
        };
        
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return sessions,
        };
        
        let session_manager: IAudioSessionManager2 = match device.Activate(
            CLSCTX_ALL,
            None,
        ) {
            Ok(m) => m,
            Err(_) => return sessions,
        };
        
        let session_enumerator = match session_manager.GetSessionEnumerator() {
            Ok(e) => e,
            Err(_) => return sessions,
        };
        
        let count = match session_enumerator.GetCount() {
            Ok(c) => c,
            Err(_) => return sessions,
        };
        
        for i in 0..count {
            let session_control = match session_enumerator.GetSession(i) {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let session_control2: IAudioSessionControl2 = match session_control.cast() {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let pid = match session_control2.GetProcessId() {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            if pid == 0 {
                continue;
            }
            
            let mut name = get_process_name(pid);
            if name.is_empty() {
                name = format!("Process {}", pid);
            }
            
            let simple_volume: ISimpleAudioVolume = match session_control2.cast() {
                Ok(v) => v,
                Err(_) => continue,
            };
            
            let volume = simple_volume.GetMasterVolume().unwrap_or(1.0);
            let mute = simple_volume.GetMute().unwrap_or(false.into()).as_bool();
            
            // Avoid duplicate entries
            if !sessions.iter().any(|s: &AudioSessionInfo| s.pid == pid) {
                sessions.push(AudioSessionInfo {
                    name,
                    pid,
                    volume,
                    mute,
                });
            }
        }
    }
    sessions
}

#[cfg(windows)]
pub fn set_session_volume(pid: u32, volume: f32) {
    if pid == u32::MAX {
        set_master_volume(volume);
        return;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            Ok(e) => e,
            Err(_) => return,
        };
        
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        
        let session_manager: IAudioSessionManager2 = match device.Activate(
            CLSCTX_ALL,
            None,
        ) {
            Ok(m) => m,
            Err(_) => return,
        };
        
        let session_enumerator = match session_manager.GetSessionEnumerator() {
            Ok(e) => e,
            Err(_) => return,
        };
        
        let count = match session_enumerator.GetCount() {
            Ok(c) => c,
            Err(_) => return,
        };
        
        for i in 0..count {
            let session_control = match session_enumerator.GetSession(i) {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let session_control2: IAudioSessionControl2 = match session_control.cast() {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let s_pid = match session_control2.GetProcessId() {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            if s_pid == pid {
                let simple_volume: ISimpleAudioVolume = match session_control2.cast() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let _ = simple_volume.SetMasterVolume(volume, std::ptr::null());
                break;
            }
        }
    }
}

#[cfg(windows)]
pub fn set_session_mute(pid: u32, mute: bool) {
    if pid == u32::MAX {
        set_master_mute(mute);
        return;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            Ok(e) => e,
            Err(_) => return,
        };
        
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        
        let session_manager: IAudioSessionManager2 = match device.Activate(
            CLSCTX_ALL,
            None,
        ) {
            Ok(m) => m,
            Err(_) => return,
        };
        
        let session_enumerator = match session_manager.GetSessionEnumerator() {
            Ok(e) => e,
            Err(_) => return,
        };
        
        let count = match session_enumerator.GetCount() {
            Ok(c) => c,
            Err(_) => return,
        };
        
        for i in 0..count {
            let session_control = match session_enumerator.GetSession(i) {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let session_control2: IAudioSessionControl2 = match session_control.cast() {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            let s_pid = match session_control2.GetProcessId() {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            if s_pid == pid {
                let simple_volume: ISimpleAudioVolume = match session_control2.cast() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let _ = simple_volume.SetMute(mute.into(), std::ptr::null());
                break;
            }
        }
    }
}

#[cfg(windows)]
pub fn get_master_volume() -> f32 {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return 1.0,
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return 1.0,
        };
        let endpoint_volume: IAudioEndpointVolume = match device.Activate(CLSCTX_ALL, None) {
            Ok(v) => v,
            Err(_) => return 1.0,
        };
        endpoint_volume.GetMasterVolumeLevelScalar().unwrap_or(1.0)
    }
}

#[cfg(windows)]
pub fn set_master_volume(volume: f32) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return,
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        let endpoint_volume: IAudioEndpointVolume = match device.Activate(CLSCTX_ALL, None) {
            Ok(v) => v,
            Err(_) => return,
        };
        let _ = endpoint_volume.SetMasterVolumeLevelScalar(volume, std::ptr::null());
    }
}

#[cfg(windows)]
pub fn get_master_mute() -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return false,
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let endpoint_volume: IAudioEndpointVolume = match device.Activate(CLSCTX_ALL, None) {
            Ok(v) => v,
            Err(_) => return false,
        };
        endpoint_volume.GetMute().unwrap_or(false.into()).as_bool()
    }
}

#[cfg(windows)]
pub fn set_master_mute(mute: bool) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return,
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        let endpoint_volume: IAudioEndpointVolume = match device.Activate(CLSCTX_ALL, None) {
            Ok(v) => v,
            Err(_) => return,
        };
        let _ = endpoint_volume.SetMute(mute.into(), std::ptr::null());
    }
}

#[cfg(not(windows))]
pub fn get_master_volume() -> f32 { 1.0 }
#[cfg(not(windows))]
pub fn set_master_volume(_volume: f32) {}
#[cfg(not(windows))]
pub fn get_master_mute() -> bool { false }
#[cfg(not(windows))]
pub fn set_master_mute(_mute: bool) {}

#[cfg(not(windows))]
pub fn get_active_sessions() -> Vec<AudioSessionInfo> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn set_session_volume(_pid: u32, _volume: f32) {}

#[cfg(not(windows))]
pub fn set_session_mute(_pid: u32, _mute: bool) {}

#[derive(Debug, Clone, Copy)]
pub enum MixerCommand {
    SetVolume { pid: u32, volume: f32 },
    SetMute { pid: u32, mute: bool },
    ForcePoll,
}
