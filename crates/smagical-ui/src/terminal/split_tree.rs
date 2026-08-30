//! 终端多窗格分屏拓扑树与自适应几何布局引擎。
//!
//! 支持任意层级与嵌套深度的二叉分屏 (水平/垂直切分)、递归几何占比推导、动态尺寸调节与窗格生命周期管理。

/// 分屏切分方向枚举。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitOrientation {
    /// 左右垂直分割 (新增左右窗格)
    Vertical,
    /// 上下水平分割 (新增上下窗格)
    Horizontal,
}

/// 单个叶子窗格经过递归推导后的屏幕归一化几何布局信息。
#[derive(Clone, Debug, PartialEq)]
pub struct PaneComputedLayout {
    /// 窗格唯一标识 ID
    pub pane_id: String,
    /// 窗格顶部标题文本
    pub title: String,
    /// 相对终端主视口左上角 X 轴起始位置占比 (0.0 ~ 1.0)
    pub x_ratio: f32,
    /// 相对终端主视口左上角 Y 轴起始位置占比 (0.0 ~ 1.0)
    pub y_ratio: f32,
    /// 相对终端主视口总宽度占比 (0.0 ~ 1.0)
    pub w_ratio: f32,
    /// 相对终端主视口总高度占比 (0.0 ~ 1.0)
    pub h_ratio: f32,
}

/// 窗格间可拖拽分割条的归一化几何布局信息。
#[derive(Clone, Debug, PartialEq)]
pub struct SplitterComputedLayout {
    /// 分割条唯一标识 ID (对应二叉树分支节点 ID)
    pub splitter_id: String,
    /// 是否为垂直分割条 (true 为左右垂直拖拽条, false 为上下水平拖拽条)
    pub is_vertical: bool,
    /// 相对终端主视口左上角 X 轴位置占比 (0.0 ~ 1.0)
    pub x_ratio: f32,
    /// 相对终端主视口左上角 Y 轴位置占比 (0.0 ~ 1.0)
    pub y_ratio: f32,
    /// 分割条沿 X 轴的跨度占比 (水平分割条有效)
    pub w_ratio: f32,
    /// 分割条沿 Y 轴的跨度占比 (垂直分割条有效)
    pub h_ratio: f32,
}

/// 终端窗格计算后物理像素几何布局。
#[derive(Clone, Debug, PartialEq)]
pub struct PanePixelLayout {
    /// 窗格唯一标识 ID
    pub pane_id: String,
    /// 窗格顶部标题文本
    pub title: String,
    /// 相对终端主视口左上角像素 X 坐标
    pub x: f32,
    /// 相对终端主视口左上角像素 Y 坐标
    pub y: f32,
    /// 窗格像素宽度
    pub width: f32,
    /// 窗格像素高度
    pub height: f32,
}

/// 分割条计算后物理像素几何布局。
#[derive(Clone, Debug, PartialEq)]
pub struct SplitterPixelLayout {
    /// 分割条唯一标识 ID (对应二叉树分支节点 ID)
    pub splitter_id: String,
    /// 是否为垂直分割条 (true 为左右垂直拖拽条, false 为上下水平拖拽条)
    pub is_vertical: bool,
    /// 相对终端主视口左上角像素 X 坐标
    pub x: f32,
    /// 相对终端主视口左上角像素 Y 坐标
    pub y: f32,
    /// 分割条像素宽度
    pub width: f32,
    /// 分割条像素高度
    pub height: f32,
}

/// 计算分屏几何时使用的物理像素区域矩形。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelRect {
    /// 相对左上角 X 轴偏移
    pub x: f32,
    /// 相对左上角 Y 轴偏移
    pub y: f32,
    /// 矩形像素宽度
    pub w: f32,
    /// 矩形像素高度
    pub h: f32,
}



/// 终端多窗格二叉分屏树节点。
#[derive(Clone, Debug, PartialEq)]
pub enum SplitNode {
    /// 叶子节点 (承载具体的终端会话/窗格)
    Leaf {
        /// 窗格唯一标识 ID
        pane_id: String,
        /// 窗格标题文本
        title: String,
    },
    /// 分支节点 (二叉分割容器)
    Branch {
        /// 分支节点唯一标识 ID
        node_id: String,
        /// 分割方向 (垂直 / 水平)
        orientation: SplitOrientation,
        /// 分割比例 (前一个子节点的尺寸占比，默认 0.5，范围 [0.15, 0.85])
        ratio: f32,
        /// 左侧或上方第一个子节点
        first: Box<SplitNode>,
        /// 右侧或下方第二个子节点
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    /// 创建单个全屏叶子窗格节点。
    ///
    /// # 参数
    /// - `pane_id`: 窗格唯一 ID
    /// - `title`: 窗格标题
    pub fn new_single(pane_id: String, title: String) -> Self {
        SplitNode::Leaf { pane_id, title }
    }

    /// 在指定的叶子窗格上执行再切分操作。
    ///
    /// # 参数
    /// - `target_pane_id`: 待切分的既有窗格 ID
    /// - `new_pane_id`: 新增的窗格 ID
    /// - `new_title`: 新增窗格的标题
    /// - `orientation`: 切分方向 (垂直左右 / 水平上下)
    ///
    /// # 返回值
    /// 若找到目标窗格并成功切分返回 `true`，否则返回 `false`。
    pub fn split_pane(
        &mut self,
        target_pane_id: &str,
        new_pane_id: String,
        new_title: String,
        orientation: SplitOrientation,
    ) -> bool {
        match self {
            SplitNode::Leaf { pane_id, title } => {
                if pane_id == target_pane_id {
                    let branch_id = format!("branch-{}-{}", target_pane_id, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
                    let old_leaf = SplitNode::Leaf {
                        pane_id: pane_id.clone(),
                        title: title.clone(),
                    };

                    let new_leaf = SplitNode::Leaf {
                        pane_id: new_pane_id,
                        title: new_title,
                    };
                    *self = SplitNode::Branch {
                        node_id: branch_id,
                        orientation,
                        ratio: 0.5,
                        first: Box::new(old_leaf),
                        second: Box::new(new_leaf),
                    };
                    true
                } else {
                    false
                }
            }
            SplitNode::Branch { first, second, .. } => {
                if first.split_pane(target_pane_id, new_pane_id.clone(), new_title.clone(), orientation) {
                    true
                } else {
                    second.split_pane(target_pane_id, new_pane_id, new_title, orientation)
                }
            }
        }
    }

    /// 关闭并移除指定的叶子窗格，将其空间自动归还给同级兄弟窗格。
    ///
    /// # 参数
    /// - `target_pane_id`: 待关闭的窗格 ID
    ///
    /// # 返回值
    /// 若成功关闭并合并返回 `true`，若窗格为最后一个全屏窗格或未找到返回 `false`。
    pub fn close_pane(&mut self, target_pane_id: &str) -> bool {
        match self {
            SplitNode::Leaf { .. } => false,
            SplitNode::Branch { first, second, .. } => {
                // 检查第一子节点是否正是目标
                if let SplitNode::Leaf { pane_id, .. } = first.as_ref()
                    && pane_id == target_pane_id
                {
                    let remaining = (**second).clone();
                    *self = remaining;
                    return true;
                }
                // 检查第二子节点是否正是目标
                if let SplitNode::Leaf { pane_id, .. } = second.as_ref()
                    && pane_id == target_pane_id
                {
                    let remaining = (**first).clone();
                    *self = remaining;
                    return true;
                }


                // 递归向下查找并关闭
                if first.close_pane(target_pane_id) {
                    true
                } else {
                    second.close_pane(target_pane_id)
                }
            }
        }
    }

    /// 动态调节指定分割条的比例。
    ///
    /// # 参数
    /// - `splitter_id`: 目标分割条 ID
    /// - `delta_ratio`: 拖拽产生的比例增量 (例如 +0.02 或 -0.01)
    pub fn adjust_splitter(&mut self, splitter_id: &str, delta_ratio: f32) -> bool {
        match self {
            SplitNode::Leaf { .. } => false,
            SplitNode::Branch {
                node_id,
                ratio,
                first,
                second,
                ..
            } => {
                if node_id == splitter_id {
                    *ratio = (*ratio + delta_ratio).clamp(0.15, 0.85);
                    true
                } else if first.adjust_splitter(splitter_id, delta_ratio) {
                    true
                } else {
                    second.adjust_splitter(splitter_id, delta_ratio)
                }
            }
        }
    }

    /// 递归计算全部分割树中叶子窗格与分割条的屏幕归一化几何布局。
    ///
    /// # 返回值
    /// `(Vec<PaneComputedLayout>, Vec<SplitterComputedLayout>)`
    pub fn compute_layout(&self) -> (Vec<PaneComputedLayout>, Vec<SplitterComputedLayout>) {
        let mut panes = Vec::new();
        let mut splitters = Vec::new();
        self.compute_recursive(0.0, 0.0, 1.0, 1.0, &mut panes, &mut splitters);
        (panes, splitters)
    }

    /// 内部递归几何推导函数。
    fn compute_recursive(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        panes: &mut Vec<PaneComputedLayout>,
        splitters: &mut Vec<SplitterComputedLayout>,
    ) {
        match self {
            SplitNode::Leaf { pane_id, title } => {
                panes.push(PaneComputedLayout {
                    pane_id: pane_id.clone(),
                    title: title.clone(),
                    x_ratio: x,
                    y_ratio: y,
                    w_ratio: w,
                    h_ratio: h,
                });
            }
            SplitNode::Branch {
                node_id,
                orientation,
                ratio,
                first,
                second,
            } => {
                match orientation {
                    SplitOrientation::Vertical => {
                        let w_first = w * ratio;
                        let w_second = w * (1.0 - ratio);
                        let splitter_x = x + w_first;

                        splitters.push(SplitterComputedLayout {
                            splitter_id: node_id.clone(),
                            is_vertical: true,
                            x_ratio: splitter_x,
                            y_ratio: y,
                            w_ratio: 0.0,
                            h_ratio: h,
                        });

                        first.compute_recursive(x, y, w_first, h, panes, splitters);
                        second.compute_recursive(splitter_x, y, w_second, h, panes, splitters);
                    }
                    SplitOrientation::Horizontal => {
                        let h_first = h * ratio;
                        let h_second = h * (1.0 - ratio);
                        let splitter_y = y + h_first;

                        splitters.push(SplitterComputedLayout {
                            splitter_id: node_id.clone(),
                            is_vertical: false,
                            x_ratio: x,
                            y_ratio: splitter_y,
                            w_ratio: w,
                            h_ratio: 0.0,
                        });

                        first.compute_recursive(x, y, w, h_first, panes, splitters);
                        second.compute_recursive(x, splitter_y, w, h_second, panes, splitters);
                    }
                }
            }
        }
    }

    /// 递归计算全部分割树中叶子窗格与分割条的精确像素几何布局。
    ///
    /// # 参数
    /// - `width`: 终端主视口像素总宽度
    /// - `height`: 终端主视口像素总高度
    /// - `splitter_thickness`: 分割条厚度 (像素, 建议 4.0 ~ 6.0)
    pub fn compute_pixel_layout(
        &self,
        width: f32,
        height: f32,
        splitter_thickness: f32,
    ) -> (Vec<PanePixelLayout>, Vec<SplitterPixelLayout>) {
        let mut panes = Vec::new();
        let mut splitters = Vec::new();
        let initial_rect = PixelRect { x: 0.0, y: 0.0, w: width, h: height };
        self.compute_pixel_recursive(initial_rect, splitter_thickness, &mut panes, &mut splitters);
        (panes, splitters)
    }

    fn compute_pixel_recursive(
        &self,
        rect: PixelRect,
        st: f32,
        panes: &mut Vec<PanePixelLayout>,
        splitters: &mut Vec<SplitterPixelLayout>,
    ) {
        match self {
            SplitNode::Leaf { pane_id, title } => {
                panes.push(PanePixelLayout {
                    pane_id: pane_id.clone(),
                    title: title.clone(),
                    x: rect.x,
                    y: rect.y,
                    width: rect.w.max(20.0),
                    height: rect.h.max(20.0),
                });
            }
            SplitNode::Branch {
                node_id,
                orientation,
                ratio,
                first,
                second,
            } => {
                match orientation {
                    SplitOrientation::Vertical => {
                        let avail_w = (rect.w - st).max(20.0);
                        let w_first = (avail_w * ratio).max(10.0);
                        let w_second = (avail_w - w_first).max(10.0);
                        let splitter_x = rect.x + w_first;

                        splitters.push(SplitterPixelLayout {
                            splitter_id: node_id.clone(),
                            is_vertical: true,
                            x: splitter_x,
                            y: rect.y,
                            width: st,
                            height: rect.h,
                        });

                        first.compute_pixel_recursive(
                            PixelRect { x: rect.x, y: rect.y, w: w_first, h: rect.h },
                            st,
                            panes,
                            splitters,
                        );
                        second.compute_pixel_recursive(
                            PixelRect { x: splitter_x + st, y: rect.y, w: w_second, h: rect.h },
                            st,
                            panes,
                            splitters,
                        );
                    }
                    SplitOrientation::Horizontal => {
                        let avail_h = (rect.h - st).max(20.0);
                        let h_first = (avail_h * ratio).max(10.0);
                        let h_second = (avail_h - h_first).max(10.0);
                        let splitter_y = rect.y + h_first;

                        splitters.push(SplitterPixelLayout {
                            splitter_id: node_id.clone(),
                            is_vertical: false,
                            x: rect.x,
                            y: splitter_y,
                            width: rect.w,
                            height: st,
                        });

                        first.compute_pixel_recursive(
                            PixelRect { x: rect.x, y: rect.y, w: rect.w, h: h_first },
                            st,
                            panes,
                            splitters,
                        );
                        second.compute_pixel_recursive(
                            PixelRect { x: rect.x, y: splitter_y + st, w: rect.w, h: h_second },
                            st,
                            panes,
                            splitters,
                        );
                    }
                }
            }
        }
    }


    /// 获取分屏树内全部活跃叶子窗格 ID 列表。
    pub fn all_pane_ids(&self) -> Vec<String> {

        let mut ids = Vec::new();
        self.collect_pane_ids(&mut ids);
        ids
    }

    fn collect_pane_ids(&self, ids: &mut Vec<String>) {
        match self {
            SplitNode::Leaf { pane_id, .. } => ids.push(pane_id.clone()),
            SplitNode::Branch { first, second, .. } => {
                first.collect_pane_ids(ids);
                second.collect_pane_ids(ids);
            }
        }
    }

    /// 获取分屏树叶子窗格总数量。
    pub fn leaf_count(&self) -> usize {
        match self {
            SplitNode::Leaf { .. } => 1,
            SplitNode::Branch { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_pane_layout() {
        let tree = SplitNode::new_single("p1".into(), "Pane 1".into());
        assert_eq!(tree.leaf_count(), 1);
        let (panes, splitters) = tree.compute_layout();
        assert_eq!(panes.len(), 1);
        assert_eq!(splitters.len(), 0);
        assert_eq!(panes[0].x_ratio, 0.0);
        assert_eq!(panes[0].w_ratio, 1.0);
    }

    #[test]
    fn test_nested_multi_split_layout() {
        let mut tree = SplitNode::new_single("p1".into(), "Pane 1".into());

        // 垂直左右切分 p1 -> p1 (左) + p2 (右)
        assert!(tree.split_pane("p1", "p2".into(), "Pane 2".into(), SplitOrientation::Vertical));
        assert_eq!(tree.leaf_count(), 2);

        // 水平上下切分 p2 -> p2 (上) + p3 (下)
        assert!(tree.split_pane("p2", "p3".into(), "Pane 3".into(), SplitOrientation::Horizontal));
        assert_eq!(tree.leaf_count(), 3);

        let (panes, splitters) = tree.compute_layout();
        assert_eq!(panes.len(), 3);
        assert_eq!(splitters.len(), 2);

        // p1: 宽 50%, 高 100%
        assert_eq!(panes[0].pane_id, "p1");
        assert_eq!(panes[0].w_ratio, 0.5);
        assert_eq!(panes[0].h_ratio, 1.0);

        // p2: X 50%, Y 0%, 宽 50%, 高 50%
        assert_eq!(panes[1].pane_id, "p2");
        assert_eq!(panes[1].x_ratio, 0.5);
        assert_eq!(panes[1].y_ratio, 0.0);
        assert_eq!(panes[1].w_ratio, 0.5);
        assert_eq!(panes[1].h_ratio, 0.5);

        // p3: X 50%, Y 50%, 宽 50%, 高 50%
        assert_eq!(panes[2].pane_id, "p3");
        assert_eq!(panes[2].x_ratio, 0.5);
        assert_eq!(panes[2].y_ratio, 0.5);
        assert_eq!(panes[2].w_ratio, 0.5);
        assert_eq!(panes[2].h_ratio, 0.5);
    }

    #[test]
    fn test_close_and_merge_pane() {
        let mut tree = SplitNode::new_single("p1".into(), "Pane 1".into());
        tree.split_pane("p1", "p2".into(), "Pane 2".into(), SplitOrientation::Vertical);
        tree.split_pane("p2", "p3".into(), "Pane 3".into(), SplitOrientation::Horizontal);
        assert_eq!(tree.leaf_count(), 3);

        // 关闭 p3，p2 自动回占整个右半区
        assert!(tree.close_pane("p3"));
        assert_eq!(tree.leaf_count(), 2);

        let (panes, _) = tree.compute_layout();
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[1].pane_id, "p2");
        assert_eq!(panes[1].h_ratio, 1.0);
    }

    #[test]
    fn test_nested_infinite_splits_and_pixel_layout() {
        let mut tree = SplitNode::new_single("p1".into(), "Pane 1".into());
        // p1 -> p1 (Left) + p2 (Right)
        tree.split_pane("p1", "p2".into(), "Pane 2".into(), SplitOrientation::Vertical);
        // p2 -> p2 (Top-Right) + p3 (Bottom-Right)
        tree.split_pane("p2", "p3".into(), "Pane 3".into(), SplitOrientation::Horizontal);
        // p3 -> p3 (Bottom-Right-Left) + p4 (Bottom-Right-Right)
        tree.split_pane("p3", "p4".into(), "Pane 4".into(), SplitOrientation::Vertical);
        // p4 -> p4 (Bottom-Right-Right-Top) + p5 (Bottom-Right-Right-Bottom)
        tree.split_pane("p4", "p5".into(), "Pane 5".into(), SplitOrientation::Horizontal);

        assert_eq!(tree.leaf_count(), 5);
        let (panes, splitters) = tree.compute_pixel_layout(1000.0, 800.0, 6.0);
        assert_eq!(panes.len(), 5);
        assert_eq!(splitters.len(), 4);

        // 验证所有窗格均有正向尺寸
        for p in &panes {
            assert!(p.width > 20.0);
            assert!(p.height > 20.0);
        }

        // 验证关闭深层嵌套节点
        assert!(tree.close_pane("p5"));
        assert_eq!(tree.leaf_count(), 4);
        assert!(tree.close_pane("p4"));
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.close_pane("p3"));
        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.close_pane("p2"));
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.all_pane_ids(), vec!["p1"]);
    }
}

