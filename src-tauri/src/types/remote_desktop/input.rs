use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    /// 0=left, 1=middle, 2=right, 3=scroll_up, 4=scroll_down
    pub button: u8,
    pub pressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEvent {
    /// X11 keysym value
    pub key_code: u32,
    pub pressed: bool,
}
