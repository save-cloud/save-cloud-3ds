use super::list_wrap_display_status::ScrollAction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ListState {
    pub top_row: i32,
    pub selected_idx: i32,
    pub display_row: i32,
}

impl ListState {
    pub fn new(display_row: i32) -> ListState {
        ListState {
            top_row: 0,
            selected_idx: 0,
            display_row,
        }
    }

    pub fn do_scroll(&mut self, size: i32, action: ScrollAction) {
        match action {
            ScrollAction::Up => {
                self.selected_idx -= 1;
            }
            ScrollAction::Down => {
                self.selected_idx += 1;
            }
            _ => {}
        }

        // selected_idx scope check
        if self.selected_idx < 0 {
            self.selected_idx = if size > 0 { size - 1 } else { 0 };
        } else if self.selected_idx >= size {
            self.selected_idx = 0;
        }
        // top_row scope check
        if self.selected_idx < self.top_row {
            self.top_row = self.selected_idx;
        } else if self.selected_idx - self.top_row >= self.display_row {
            self.top_row = self.selected_idx - self.display_row + 1;
        }
    }

    pub fn update(&mut self, size: i32) {
        if self.selected_idx > size - 1 {
            self.selected_idx = size - 1;
        } else if self.selected_idx < 0 {
            self.selected_idx = 0;
        }
    }

    pub fn set_selected_idx(&mut self, idx: i32) {
        self.selected_idx = idx;
    }
}
