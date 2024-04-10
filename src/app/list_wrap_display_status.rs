pub enum ScrollAction {
    Up,
    Down,
    Left,
    Right,
}

pub struct ListDisplayStatus {
    pub top_row: usize,
    pub selected_idx: usize,
    pub column: usize,
    pub row: usize,
}

impl ListDisplayStatus {
    pub fn new(column: usize, row: usize) -> Self {
        Self {
            top_row: 0,
            selected_idx: 0,
            column,
            row,
        }
    }

    pub fn do_action(&mut self, size: usize, scroll_action: ScrollAction) {
        let idx = self.selected_idx;
        let top = self.top_row;
        match scroll_action {
            ScrollAction::Left => {
                if idx > 0 {
                    self.selected_idx = idx - 1;
                }
                if self.selected_idx < self.column * top && top > 0 {
                    self.top_row -= 1;
                }
            }
            ScrollAction::Right => {
                if idx < size - 1 {
                    self.selected_idx += 1;
                }
                if self.selected_idx - top * self.column >= self.column * self.row {
                    self.top_row += 1;
                }
            }
            ScrollAction::Up => {
                if idx / self.column == 0 {
                    let rows = (size - 1) / self.column + 1;
                    self.selected_idx = self.selected_idx % self.column + (rows - 1) * self.column;
                    if self.selected_idx >= size {
                        self.selected_idx = size - 1;
                    }
                    self.top_row = if rows >= self.row { rows - self.row } else { 0 };
                } else if self.selected_idx >= self.column {
                    self.selected_idx -= self.column;
                    // scroll down
                    if self.selected_idx < self.column * self.top_row {
                        self.top_row -= 1;
                    }
                }
            }
            ScrollAction::Down => {
                if (idx + self.column) / self.column > (size - 1) / self.column {
                    self.selected_idx %= self.column;
                    self.top_row = 0;
                } else if idx + self.column < size {
                    self.selected_idx += self.column;
                    // scroll up
                    if self.selected_idx - self.top_row * self.column >= self.column * self.row {
                        self.top_row += 1;
                    }
                } else if idx % self.column > (size - 1) % self.column {
                    self.selected_idx = size - 1;
                    // scroll up
                    if self.selected_idx - self.top_row * self.column >= self.column * self.row {
                        self.top_row += 1;
                    }
                }
            }
        };
    }
}
