use serde::{Deserialize, Serialize};

/// A user-bound hotkey — stores any virtual key code + display name and modifier flags
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyBinding {
    pub vk_code: i32,
    pub name: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            vk_code: 0xA2, // VK_LCONTROL
            name: "Left CTRL".to_string(),
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
}

impl HotkeyBinding {
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

fn is_ctrl_key(vk: i32) -> bool {
    vk == 0x11 || vk == 0xA2 || vk == 0xA3
}

fn is_alt_key(vk: i32) -> bool {
    vk == 0x12 || vk == 0xA4 || vk == 0xA5
}

fn is_shift_key(vk: i32) -> bool {
    vk == 0x10 || vk == 0xA0 || vk == 0xA1
}

fn is_modifier(vk: i32) -> bool {
    matches!(vk, 0x10..=0x12 | 0xA0..=0xA5)
}

/// Check if the configured hotkey is currently held down
pub fn is_hotkey_held(hotkey: &HotkeyBinding) -> bool {
    #[cfg(windows)]
    {
        unsafe {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
            
            // Check modifier states
            let ctrl_down = (GetAsyncKeyState(0x11) & (0x8000u16 as i16)) != 0;
            let alt_down = (GetAsyncKeyState(0x12) & (0x8000u16 as i16)) != 0;
            let shift_down = (GetAsyncKeyState(0x10) & (0x8000u16 as i16)) != 0;
            
            if !is_ctrl_key(hotkey.vk_code) && ctrl_down != hotkey.ctrl {
                return false;
            }
            if !is_alt_key(hotkey.vk_code) && alt_down != hotkey.alt {
                return false;
            }
            if !is_shift_key(hotkey.vk_code) && shift_down != hotkey.shift {
                return false;
            }
            
            let state = GetAsyncKeyState(hotkey.vk_code);
            state & (0x8000u16 as i16) != 0
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hotkey;
        false
    }
}

/// All scannable keys: (vk_code, display_name)
const SCANNABLE_KEYS: &[(i32, &str)] = &[
    // F-keys first (common hotkey choices)
    (0x70, "F1"), (0x71, "F2"), (0x72, "F3"), (0x73, "F4"),
    (0x74, "F5"), (0x75, "F6"), (0x76, "F7"), (0x77, "F8"),
    (0x78, "F9"), (0x79, "F10"), (0x7A, "F11"), (0x7B, "F12"),
    // Modifiers
    (0xA2, "Left CTRL"), (0xA3, "Right CTRL"),
    (0xA4, "Left ALT"), (0xA5, "Right ALT"),
    (0xA0, "Left SHIFT"), (0xA1, "Right SHIFT"),
    // Special
    (0x14, "CAPS LOCK"), (0x09, "TAB"), (0x20, "SPACE"),
    (0x2D, "INSERT"), (0x2E, "DELETE"), (0x24, "HOME"),
    (0x23, "END"), (0x21, "PAGE UP"), (0x22, "PAGE DOWN"),
    (0xC0, "` (Tilde)"), (0xDC, "\\ (Backslash)"),
    (0xDB, "[ (Left Bracket)"), (0xDD, "] (Right Bracket)"),
    (0xBA, "; (Semicolon)"), (0xDE, "' (Quote)"),
    (0xBC, ", (Comma)"), (0xBE, ". (Period)"),
    (0xBF, "/ (Slash)"), (0xBD, "- (Minus)"), (0xBB, "= (Equals)"),
    // Letters
    (0x41, "A"), (0x42, "B"), (0x43, "C"), (0x44, "D"),
    (0x45, "E"), (0x46, "F"), (0x47, "G"), (0x48, "H"),
    (0x49, "I"), (0x4A, "J"), (0x4B, "K"), (0x4C, "L"),
    (0x4D, "M"), (0x4E, "N"), (0x4F, "O"), (0x50, "P"),
    (0x51, "Q"), (0x52, "R"), (0x53, "S"), (0x54, "T"),
    (0x55, "U"), (0x56, "V"), (0x57, "W"), (0x58, "X"),
    (0x59, "Y"), (0x5A, "Z"),
    // Numbers
    (0x30, "0"), (0x31, "1"), (0x32, "2"), (0x33, "3"),
    (0x34, "4"), (0x35, "5"), (0x36, "6"), (0x37, "7"),
    (0x38, "8"), (0x39, "9"),
    // Numpad
    (0x60, "Numpad 0"), (0x61, "Numpad 1"), (0x62, "Numpad 2"),
    (0x63, "Numpad 3"), (0x64, "Numpad 4"), (0x65, "Numpad 5"),
    (0x66, "Numpad 6"), (0x67, "Numpad 7"), (0x68, "Numpad 8"),
    (0x69, "Numpad 9"),
];

/// Scan all common keys, return the first one currently pressed
pub fn detect_pressed_key() -> Option<HotkeyBinding> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        
        let ctrl_down = (unsafe { GetAsyncKeyState(0x11) } & (0x8000u16 as i16)) != 0;
        let alt_down = (unsafe { GetAsyncKeyState(0x12) } & (0x8000u16 as i16)) != 0;
        let shift_down = (unsafe { GetAsyncKeyState(0x10) } & (0x8000u16 as i16)) != 0;
        
        // 1. Search for any active non-modifier key
        for &(vk, name) in SCANNABLE_KEYS {
            if is_modifier(vk) {
                continue;
            }
            let state = unsafe { GetAsyncKeyState(vk) };
            if state & (0x8000u16 as i16) != 0 {
                let mut parts = Vec::new();
                if ctrl_down {
                    parts.push("Ctrl");
                }
                if alt_down {
                    parts.push("Alt");
                }
                if shift_down {
                    parts.push("Shift");
                }
                parts.push(name);
                let full_name = parts.join(" + ");
                return Some(HotkeyBinding {
                    vk_code: vk,
                    name: full_name,
                    ctrl: ctrl_down,
                    alt: alt_down,
                    shift: shift_down,
                });
            }
        }
        
        // 2. If no non-modifier key is pressed, check if a modifier key itself is pressed
        for &(vk, name) in SCANNABLE_KEYS {
            if !is_modifier(vk) {
                continue;
            }
            let state = unsafe { GetAsyncKeyState(vk) };
            if state & (0x8000u16 as i16) != 0 {
                return Some(HotkeyBinding {
                    vk_code: vk,
                    name: name.to_string(),
                    ctrl: false,
                    alt: false,
                    shift: false,
                });
            }
        }
        
        None
    }
    #[cfg(not(windows))]
    {
        None
    }
}
