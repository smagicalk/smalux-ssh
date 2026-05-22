//! 工作区背景轮播状态操作。

use super::WorkspaceUiState;

impl WorkspaceUiState {
    /// 切换到下一张背景。
    pub fn next_background(&mut self, source_count: usize) {
        if source_count == 0 {
            self.background_carousel.active_index = 0;
            return;
        }

        self.background_carousel.active_index =
            (self.background_carousel.active_index + 1) % source_count;
    }

    /// 返回当前背景索引，自动限制到来源数量内。
    pub fn active_background_index(&self, source_count: usize) -> Option<usize> {
        if source_count == 0 || !self.background_carousel.enabled {
            None
        } else {
            Some(self.background_carousel.active_index % source_count)
        }
    }
}
