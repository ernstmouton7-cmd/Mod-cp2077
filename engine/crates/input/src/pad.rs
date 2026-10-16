//! Gamepad model (Xbox-style standard layout) and button glyphs.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadButton {
    /// A (Xbox) / Cross (PlayStation)
    South,
    /// B / Circle
    East,
    /// X / Square
    West,
    /// Y / Triangle
    North,
    LeftBumper,
    RightBumper,
    /// View / Share / Select
    View,
    /// Menu / Options / Start
    Menu,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Guide,
}

impl PadButton {
    pub const COUNT: usize = 15;
    pub const ALL: [PadButton; 15] = [
        PadButton::South,
        PadButton::East,
        PadButton::West,
        PadButton::North,
        PadButton::LeftBumper,
        PadButton::RightBumper,
        PadButton::View,
        PadButton::Menu,
        PadButton::LeftStick,
        PadButton::RightStick,
        PadButton::DpadUp,
        PadButton::DpadDown,
        PadButton::DpadLeft,
        PadButton::DpadRight,
        PadButton::Guide,
    ];
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadAxis {
    LeftX,
    /// Up is positive.
    LeftY,
    RightX,
    RightY,
    /// 0..1
    LeftTrigger,
    /// 0..1
    RightTrigger,
}

impl PadAxis {
    pub const COUNT: usize = 6;
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PadKind {
    #[default]
    Xbox,
    PlayStation,
    Nintendo,
    Generic,
}

impl PadKind {
    /// Guess from a device name.
    pub fn from_name(name: &str) -> Self {
        let n = name.to_ascii_lowercase();
        if n.contains("xbox") || n.contains("xinput") || n.contains("x-box") {
            PadKind::Xbox
        } else if n.contains("dualsense")
            || n.contains("dualshock")
            || n.contains("playstation")
            || n.contains("ps4")
            || n.contains("ps5")
            || n.contains("sony")
        {
            PadKind::PlayStation
        } else if n.contains("nintendo") || n.contains("switch") || n.contains("joy-con") {
            PadKind::Nintendo
        } else {
            PadKind::Generic
        }
    }
}

/// Snapshot of one gamepad.
#[derive(Clone, Debug, Default)]
pub struct GamepadState {
    pub connected: bool,
    pub name: String,
    pub kind: PadKind,
    pub buttons: [bool; PadButton::COUNT],
    pub axes: [f32; PadAxis::COUNT],
}

impl GamepadState {
    #[inline]
    pub fn button(&self, b: PadButton) -> bool {
        self.buttons[b.index()]
    }
    #[inline]
    pub fn axis(&self, a: PadAxis) -> f32 {
        self.axes[a.index()]
    }
    pub fn set_button(&mut self, b: PadButton, down: bool) {
        self.buttons[b.index()] = down;
    }
    pub fn set_axis(&mut self, a: PadAxis, v: f32) {
        self.axes[a.index()] = v;
    }
    /// Any input beyond noise (to detect which device the player is using).
    pub fn active(&self) -> bool {
        self.buttons.iter().any(|b| *b)
            || self.axes[0].abs() > 0.3
            || self.axes[1].abs() > 0.3
            || self.axes[2].abs() > 0.3
            || self.axes[3].abs() > 0.3
            || self.axes[4] > 0.15
            || self.axes[5] > 0.15
    }
}

/// On-screen name for a button on a given controller family.
pub fn button_label(kind: PadKind, b: PadButton) -> &'static str {
    use PadButton::*;
    match (kind, b) {
        (PadKind::PlayStation, South) => "✕",
        (PadKind::PlayStation, East) => "○",
        (PadKind::PlayStation, West) => "□",
        (PadKind::PlayStation, North) => "△",
        (PadKind::PlayStation, LeftBumper) => "L1",
        (PadKind::PlayStation, RightBumper) => "R1",
        (PadKind::PlayStation, View) => "Create",
        (PadKind::PlayStation, Menu) => "Options",
        (PadKind::PlayStation, LeftStick) => "L3",
        (PadKind::PlayStation, RightStick) => "R3",
        (PadKind::Nintendo, South) => "B",
        (PadKind::Nintendo, East) => "A",
        (PadKind::Nintendo, West) => "Y",
        (PadKind::Nintendo, North) => "X",
        (PadKind::Nintendo, LeftBumper) => "L",
        (PadKind::Nintendo, RightBumper) => "R",
        (PadKind::Nintendo, View) => "−",
        (PadKind::Nintendo, Menu) => "+",
        (_, South) => "A",
        (_, East) => "B",
        (_, West) => "X",
        (_, North) => "Y",
        (_, LeftBumper) => "LB",
        (_, RightBumper) => "RB",
        (_, View) => "View",
        (_, Menu) => "Menu",
        (_, LeftStick) => "LS",
        (_, RightStick) => "RS",
        (_, DpadUp) => "D-pad ↑",
        (_, DpadDown) => "D-pad ↓",
        (_, DpadLeft) => "D-pad ←",
        (_, DpadRight) => "D-pad →",
        (_, Guide) => "Guide",
    }
}

pub fn axis_label(kind: PadKind, a: PadAxis) -> &'static str {
    match (kind, a) {
        (PadKind::PlayStation, PadAxis::LeftTrigger) => "L2",
        (PadKind::PlayStation, PadAxis::RightTrigger) => "R2",
        (PadKind::Nintendo, PadAxis::LeftTrigger) => "ZL",
        (PadKind::Nintendo, PadAxis::RightTrigger) => "ZR",
        (_, PadAxis::LeftTrigger) => "LT",
        (_, PadAxis::RightTrigger) => "RT",
        (_, PadAxis::LeftX) | (_, PadAxis::LeftY) => "Left stick",
        (_, PadAxis::RightX) | (_, PadAxis::RightY) => "Right stick",
    }
}
