//! 终端多窗格分屏拓扑树与自适应几何布局引擎。
//!
//! 支持任意层级与嵌套深度的二叉分屏 (水平/垂直切分)、递归几何占比推导、动态尺寸调节与窗格生命周期管理。

use serde::{Deserialize, Serialize};

/// 分屏切分方向枚举。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// 递归查找指定窗格 ID 的标题文本。
    pub fn find_pane_title(&self, target_id: &str) -> Option<String> {
        match self {
            SplitNode::Leaf { pane_id, title } => {
                if pane_id == target_id {
                    Some(title.clone())
                } else {
                    None
                }
            }
            SplitNode::Branch { first, second, .. } => {
                first.find_pane_title(target_id).or_else(|| second.find_pane_title(target_id))
            }
        }
    }

    /// 推导当前二叉分屏树在指定像素视口中的几何布局。
    ///
    /// # 参数
    /// - `width`: 终端主视口像素总宽度
    /// - `height`: 终端主视口像素总高度
    /// - `splitter_thickness`: 分割条厚度 (像素, 建议 2.0 ~ 6.0)
    /// - `zoomed_pane_id`: 若指定窗格处于临时最大化 (Zoom) 状态，则该窗格独占 100% 视口，分割线集合为空
    pub fn compute_pixel_layout(
        &self,
        width: f32,
        height: f32,
        splitter_thickness: f32,
        zoomed_pane_id: Option<&str>,
    ) -> (Vec<PanePixelLayout>, Vec<SplitterPixelLayout>) {
        if let Some(target_id) = zoomed_pane_id
            && let Some(leaf_title) = self.find_pane_title(target_id)
        {
            return (
                vec![PanePixelLayout {
                    pane_id: target_id.to_string(),
                    title: leaf_title,
                    x: 0.0,
                    y: 0.0,
                    width: width.max(20.0),
                    height: height.max(20.0),
                }],
                Vec::new(),
            );
        }


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

/// 经典运维多分屏布局预设枚举。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitLayoutPreset {
    /// 单屏全屏 (1 窗格)
    Single,
    /// 左右等分双屏 (2 窗格, 1:1)
    DualVertical,
    /// 上下等分双屏 (2 窗格, 1:1)
    DualHorizontal,
    /// 四分田字格 (4 窗格, 2x2)
    QuadGrid,
    /// 主辅分屏：一大左两小右 (3 窗格, 1L2R)
    MainLeftDualRight,
    /// 主辅分屏：一大上两小下 (3 窗格, 1T2B)
    MainTopDualBottom,
    /// 三列并排等分 (3 窗格, 1:1:1)
    TripleColumns,
    /// 三行并排等分 (3 窗格, 1:1:1)
    TripleRows,
}

impl SplitLayoutPreset {
    /// 从字符串解析预设标识
    pub fn from_preset_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "single" | "1" => Some(Self::Single),
            "dual_vertical" | "dual-vertical" | "v" | "2v" => Some(Self::DualVertical),
            "dual_horizontal" | "dual-horizontal" | "h" | "2h" => Some(Self::DualHorizontal),
            "quad_grid" | "quad-grid" | "quad" | "grid" | "4" | "2x2" => Some(Self::QuadGrid),
            "main_left_dual_right" | "main-left-dual-right" | "1l2r" => Some(Self::MainLeftDualRight),
            "main_top_dual_bottom" | "main-top-dual-bottom" | "1t2b" => Some(Self::MainTopDualBottom),
            "triple_columns" | "triple-columns" | "3col" | "3v" => Some(Self::TripleColumns),
            "triple_rows" | "triple-rows" | "3row" | "3h" => Some(Self::TripleRows),
            _ => None,
        }
    }

    /// 获取预设英文标识名
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::DualVertical => "dual_vertical",
            Self::DualHorizontal => "dual_horizontal",
            Self::QuadGrid => "quad_grid",
            Self::MainLeftDualRight => "main_left_dual_right",
            Self::MainTopDualBottom => "main_top_dual_bottom",
            Self::TripleColumns => "triple_columns",
            Self::TripleRows => "triple_rows",
        }
    }

    /// 所需最少窗格数
    pub fn required_panes(&self) -> usize {
        match self {
            Self::Single => 1,
            Self::DualVertical | Self::DualHorizontal => 2,
            Self::MainLeftDualRight | Self::MainTopDualBottom | Self::TripleColumns | Self::TripleRows => 3,
            Self::QuadGrid => 4,
        }
    }

    /// 根据预设与传入的窗格 (ID, 标题) 列表构建二叉分屏拓扑树。
    /// 若传入的窗格列表数量不足，将自动使用默认 ID 与标题补齐。
    pub fn build_tree(&self, panes: &[(String, String)]) -> SplitNode {
        let get_pane = |idx: usize| -> (String, String) {
            if let Some((id, title)) = panes.get(idx) {
                (id.clone(), title.clone())
            } else {
                (format!("pane-{}", idx + 1), format!("Pane {}", idx + 1))
            }
        };

        match self {
            Self::Single => {
                let (id, title) = get_pane(0);
                SplitNode::new_single(id, title)
            }
            Self::DualVertical => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                SplitNode::Branch {
                    node_id: "branch-preset-v".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                }
            }
            Self::DualHorizontal => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                SplitNode::Branch {
                    node_id: "branch-preset-h".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                }
            }
            Self::QuadGrid => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                let (id3, t3) = get_pane(2);
                let (id4, t4) = get_pane(3);
                // 左侧列 (上 1 + 下 3)
                let left_col = SplitNode::Branch {
                    node_id: "branch-quad-left".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id3, title: t3 }),
                };
                // 右侧列 (上 2 + 下 4)
                let right_col = SplitNode::Branch {
                    node_id: "branch-quad-right".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id4, title: t4 }),
                };
                SplitNode::Branch {
                    node_id: "branch-quad-root".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.5,
                    first: Box::new(left_col),
                    second: Box::new(right_col),
                }
            }
            Self::MainLeftDualRight => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                let (id3, t3) = get_pane(2);
                let right_sub = SplitNode::Branch {
                    node_id: "branch-mldr-right".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id3, title: t3 }),
                };
                SplitNode::Branch {
                    node_id: "branch-mldr-root".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(right_sub),
                }
            }
            Self::MainTopDualBottom => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                let (id3, t3) = get_pane(2);
                let bottom_sub = SplitNode::Branch {
                    node_id: "branch-mtdb-bottom".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id3, title: t3 }),
                };
                SplitNode::Branch {
                    node_id: "branch-mtdb-root".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(bottom_sub),
                }
            }
            Self::TripleColumns => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                let (id3, t3) = get_pane(2);
                let right_two = SplitNode::Branch {
                    node_id: "branch-triple-col-right".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id3, title: t3 }),
                };
                SplitNode::Branch {
                    node_id: "branch-triple-col-root".to_string(),
                    orientation: SplitOrientation::Vertical,
                    ratio: 0.3333,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(right_two),
                }
            }
            Self::TripleRows => {
                let (id1, t1) = get_pane(0);
                let (id2, t2) = get_pane(1);
                let (id3, t3) = get_pane(2);
                let bottom_two = SplitNode::Branch {
                    node_id: "branch-triple-row-bottom".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf { pane_id: id2, title: t2 }),
                    second: Box::new(SplitNode::Leaf { pane_id: id3, title: t3 }),
                };
                SplitNode::Branch {
                    node_id: "branch-triple-row-root".to_string(),
                    orientation: SplitOrientation::Horizontal,
                    ratio: 0.3333,
                    first: Box::new(SplitNode::Leaf { pane_id: id1, title: t1 }),
                    second: Box::new(bottom_two),
                }
            }
        }
    }
}

/// 终端分屏拓扑存储模型 (用于持久化保存会话分屏布局)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedSplitTopology {
    /// 分屏拓扑二叉树
    pub root: SplitNode,
    /// 活跃/选中聚焦的窗格 ID
    pub active_pane_id: String,
    /// 最后保存时间戳 (UNIX 秒)
    pub updated_at: u64,
}

impl PersistedSplitTopology {
    /// 创建新的分屏拓扑存储模型
    pub fn new(root: SplitNode, active_pane_id: String) -> Self {
        let updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            root,
            active_pane_id,
            updated_at,
        }
    }

    /// 获取分屏拓扑默认持久化存储路径 (`%LOCALAPPDATA%\smagical\smalux\layouts\last_topology.json`)
    pub fn get_default_path() -> std::path::PathBuf {
        let base_dir = directories::ProjectDirs::from("com", "smagical", "smalux")
            .map(|dirs| dirs.data_local_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let layout_dir = base_dir.join("layouts");
        let _ = std::fs::create_dir_all(&layout_dir);
        layout_dir.join("last_topology.json")
    }

    /// 保存分屏拓扑到指定文件
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// 从指定文件加载分屏拓扑
    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// 异步后台持久化分屏拓扑
    pub fn save_async(&self) {
        let clone = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = Self::get_default_path();
            let _ = clone.save_to_file(&path);
        });
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
        let (panes, splitters) = tree.compute_pixel_layout(1000.0, 800.0, 6.0, None);
        assert_eq!(panes.len(), 5);
        assert_eq!(splitters.len(), 4);

        // 验证单窗格 Zoom 临时全屏
        let (zoomed_panes, zoomed_splitters) = tree.compute_pixel_layout(1000.0, 800.0, 6.0, Some("p3"));
        assert_eq!(zoomed_panes.len(), 1);
        assert_eq!(zoomed_panes[0].pane_id, "p3");
        assert_eq!(zoomed_panes[0].width, 1000.0);
        assert_eq!(zoomed_panes[0].height, 800.0);
        assert_eq!(zoomed_splitters.len(), 0);

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

    #[test]
    fn test_split_layout_presets() {
        let panes = vec![
            ("pane-a".to_string(), "Terminal A".to_string()),
            ("pane-b".to_string(), "Terminal B".to_string()),
            ("pane-c".to_string(), "Terminal C".to_string()),
            ("pane-d".to_string(), "Terminal D".to_string()),
        ];

        // 1. Single
        let t1 = SplitLayoutPreset::Single.build_tree(&panes);
        assert_eq!(t1.leaf_count(), 1);
        assert_eq!(t1.all_pane_ids(), vec!["pane-a"]);

        // 2. DualVertical
        let t2 = SplitLayoutPreset::DualVertical.build_tree(&panes);
        assert_eq!(t2.leaf_count(), 2);
        assert_eq!(t2.all_pane_ids(), vec!["pane-a", "pane-b"]);

        // 3. QuadGrid
        let t4 = SplitLayoutPreset::QuadGrid.build_tree(&panes);
        assert_eq!(t4.leaf_count(), 4);
        assert_eq!(t4.all_pane_ids(), vec!["pane-a", "pane-c", "pane-b", "pane-d"]);

        // 4. MainLeftDualRight
        let t_mldr = SplitLayoutPreset::MainLeftDualRight.build_tree(&panes);
        assert_eq!(t_mldr.leaf_count(), 3);
        assert_eq!(t_mldr.all_pane_ids(), vec!["pane-a", "pane-b", "pane-c"]);

        // 5. Preset from string
        assert_eq!(SplitLayoutPreset::from_preset_name("quad"), Some(SplitLayoutPreset::QuadGrid));
        assert_eq!(SplitLayoutPreset::from_preset_name("2v"), Some(SplitLayoutPreset::DualVertical));
        assert_eq!(SplitLayoutPreset::from_preset_name("unknown"), None);
    }

    #[test]
    fn test_persisted_split_topology_serde_roundtrip() {
        let panes = vec![
            ("pane-1".to_string(), "Host 1".to_string()),
            ("pane-2".to_string(), "Host 2".to_string()),
            ("pane-3".to_string(), "Host 3".to_string()),
        ];
        let tree = SplitLayoutPreset::MainLeftDualRight.build_tree(&panes);
        let orig = PersistedSplitTopology::new(tree, "pane-2".to_string());

        let json = serde_json::to_string(&orig).expect("Serialize topology failed");
        let decoded: PersistedSplitTopology = serde_json::from_str(&json).expect("Deserialize topology failed");

        assert_eq!(decoded.active_pane_id, "pane-2");
        assert_eq!(decoded.root.leaf_count(), 3);
        assert_eq!(decoded.root.all_pane_ids(), vec!["pane-1", "pane-2", "pane-3"]);
    }
}

