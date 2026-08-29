use core_types::metadata::ModifierState;

/// Keyboard modifier tracker tracking Ctrl, Alt, Shift, Win/Meta, CapsLock, NumLock states.
#[derive(Debug, Clone, Default)]
pub struct KeyboardModifierTracker {
    pub l_ctrl: bool,
    pub r_ctrl: bool,
    pub l_alt: bool,
    pub r_alt: bool,
    pub l_shift: bool,
    pub r_shift: bool,
    pub l_win: bool,
    pub r_win: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
}

impl KeyboardModifierTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update modifier state given a virtual key code and is_key_down flag.
    pub fn update_vk(&mut self, vk_code: u32, is_down: bool) {
        match vk_code {
            0x11 | 0xA2 => self.l_ctrl = is_down, // VK_CONTROL, VK_LCONTROL
            0xA3 => self.r_ctrl = is_down,        // VK_RCONTROL
            0x12 | 0xA4 => self.l_alt = is_down,  // VK_MENU, VK_LMENU
            0xA5 => self.r_alt = is_down,         // VK_RMENU
            0x10 | 0xA0 => self.l_shift = is_down, // VK_SHIFT, VK_LSHIFT
            0xA1 => self.r_shift = is_down,       // VK_RSHIFT
            0x5B => self.l_win = is_down,         // VK_LWIN
            0x5C => self.r_win = is_down,         // VK_RWIN
            0x14 if is_down => self.caps_lock = !self.caps_lock, // VK_CAPITAL toggle
            0x90 if is_down => self.num_lock = !self.num_lock, // VK_NUMLOCK toggle
            _ => {}
        }
    }

    /// Export current modifier state.
    pub fn current_modifiers(&self) -> ModifierState {
        ModifierState {
            ctrl: self.l_ctrl || self.r_ctrl,
            alt: self.l_alt || self.r_alt,
            shift: self.l_shift || self.r_shift,
            win: self.l_win || self.r_win,
            caps_lock: self.caps_lock,
            num_lock: self.num_lock,
        }
    }

    /// Translate virtual key code to a friendly canonical name.
    pub fn vk_to_key_name(vk_code: u32) -> String {
        match vk_code {
            0x08 => "Backspace".into(),
            0x09 => "Tab".into(),
            0x0C => "Clear".into(),
            0x0D => "Enter".into(),
            0x10 | 0xA0 | 0xA1 => "Shift".into(),
            0x11 | 0xA2 | 0xA3 => "Control".into(),
            0x12 | 0xA4 | 0xA5 => "Alt".into(),
            0x13 => "Pause".into(),
            0x14 => "CapsLock".into(),
            0x1B => "Escape".into(),
            0x20 => "Space".into(),
            0x21 => "PageUp".into(),
            0x22 => "PageDown".into(),
            0x23 => "End".into(),
            0x24 => "Home".into(),
            0x25 => "ArrowLeft".into(),
            0x26 => "ArrowUp".into(),
            0x27 => "ArrowRight".into(),
            0x28 => "ArrowDown".into(),
            0x2C => "PrintScreen".into(),
            0x2D => "Insert".into(),
            0x2E => "Delete".into(),
            0x30..=0x39 => {
                let digit = vk_code as u8;
                format!("{}", digit as char)
            }
            0x41..=0x5A => {
                let char_code = (vk_code) as u8;
                format!("{}", char_code as char)
            }
            0x5B | 0x5C => "Meta".into(),
            0x5D => "ContextMenu".into(),
            0x60..=0x69 => format!("Numpad{}", vk_code - 0x60),
            0x6A => "NumpadMultiply".into(),
            0x6B => "NumpadAdd".into(),
            0x6C => "NumpadSeparator".into(),
            0x6D => "NumpadSubtract".into(),
            0x6E => "NumpadDecimal".into(),
            0x6F => "NumpadDivide".into(),
            0x70..=0x87 => format!("F{}", vk_code - 0x70 + 1),
            0x90 => "NumLock".into(),
            0x91 => "ScrollLock".into(),
            0xBA => ";".into(),
            0xBB => "=".into(),
            0xBC => ",".into(),
            0xBD => "-".into(),
            0xBE => ".".into(),
            0xBF => "/".into(),
            0xC0 => "`".into(),
            0xDB => "[".into(),
            0xDC => "\\".into(),
            0xDD => "]".into(),
            0xDE => "'".into(),
            other => format!("VK_0x{:02X}", other),
        }
    }
}
