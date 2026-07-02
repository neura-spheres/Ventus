use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SplitMode {
    None,
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct SplitView {
    pub mode: SplitMode,
    pub primary_tab_id: Option<String>,
    pub secondary_tab_id: Option<String>,
    pub split_ratio: f32,
}

impl SplitView {
    pub fn new() -> Self {
        Self {
            mode: SplitMode::None,
            primary_tab_id: None,
            secondary_tab_id: None,
            split_ratio: 0.5,
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode != SplitMode::None
    }

    pub fn activate(&mut self, primary: String, secondary: String, mode: SplitMode) {
        self.primary_tab_id = Some(primary);
        self.secondary_tab_id = Some(secondary);
        self.mode = mode;
    }

    pub fn deactivate(&mut self) {
        self.mode = SplitMode::None;
        self.secondary_tab_id = None;
    }
}
