//! 终端渲染双缓冲 (Ping-Pong SharedPixelBuffer) 模块。
//!
//! 彻底解决 Slint 单缓冲区在 60FPS 下因 `Image::from_rgba8(buf.clone())` 导致引用计数为 2、
//! 进而在下一帧 `make_mut_bytes()` 时被迫触发 Copy-on-Write 底层深拷贝的性能瓶颈。
//!
//! 通过 Ping-Pong 双缓冲交替轮换，保证写入端始终独占当前缓冲 (Ref Count == 1)，
//! 并结合行级脏标记追踪与每缓冲哈希指纹，实现 60FPS 下整屏像素局部就地刷新与零堆内存分配。

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// 256 行紧凑脏标记位图 (4 个 u64，无堆分配，32 字节)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowDirtyMask {
    /// 紧凑 256 位位图数组，每个 bit 对应终端一行的脏标记
    pub bits: [u64; 4],
}

impl Default for RowDirtyMask {
    fn default() -> Self {
        Self::none()
    }
}

impl RowDirtyMask {
    /// 包含全量行脏标记
    #[inline(always)]
    pub fn all() -> Self {
        Self {
            bits: [!0, !0, !0, !0],
        }
    }

    /// 空脏标记 (无行变动)
    #[inline(always)]
    pub fn none() -> Self {
        Self {
            bits: [0, 0, 0, 0],
        }
    }

    /// 标记指定行脏
    #[inline(always)]
    pub fn mark(&mut self, row: usize) {
        if row < 256 {
            self.bits[row / 64] |= 1u64 << (row % 64);
        }
    }

    /// 标记闭区间行范围为脏 [start_row, end_row]
    #[inline(always)]
    pub fn mark_range(&mut self, start_row: usize, end_row: usize) {
        let (s, e) = if start_row <= end_row {
            (start_row, end_row)
        } else {
            (end_row, start_row)
        };
        for r in s..=e {
            self.mark(r);
        }
    }

    /// 查询指定行是否为脏
    #[inline(always)]
    pub fn is_dirty(&self, row: usize) -> bool {
        if row < 256 {
            (self.bits[row / 64] & (1u64 << (row % 64))) != 0
        } else {
            true
        }
    }

    /// 是否所有行均无脏标记
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.bits == [0, 0, 0, 0]
    }

    /// 是否包含全量行脏标记 (用于判断是否需要全屏刷底)
    #[inline(always)]
    pub fn is_all(&self) -> bool {
        self.bits == [!0, !0, !0, !0]
    }

    /// 标记所有行为脏
    #[inline(always)]
    pub fn mark_all(&mut self) {
        self.bits = [!0, !0, !0, !0];
    }

    /// 清空所有脏标记
    #[inline(always)]
    pub fn clear(&mut self) {
        self.bits = [0, 0, 0, 0];
    }
}

/// 具备引用隔离与行级脏标记追踪的 Ping-Pong 像素双缓冲管理器。
pub struct PingPongPixelBuffer {
    buffers: [SharedPixelBuffer<Rgba8Pixel>; 2],
    active_idx: usize,
    width: u32,
    height: u32,
    /// 每个后置缓冲区上一次渲染时的每行哈希指纹
    row_hashes: [Vec<u64>; 2],
    /// 每个后置缓冲区上一次渲染时的光标信息: (col, row, is_blink_on)
    last_cursor: [Option<(usize, usize, bool)>; 2],
    /// 每个后置缓冲区上一次渲染时的选区坐标: ((s_c, s_r), (e_c, e_r))
    last_selection: [Option<((usize, usize), (usize, usize))>; 2],
    /// 每个缓冲区是否强制要求全量重绘 (如初次初始化、尺寸改变或会话切换)
    force_full_redraw: [bool; 2],
    /// 绑定的终端会话 ID
    session_id: String,
    /// 渲染器代际追踪 (调色板/字体变更使双缓冲统一失效)
    renderer_generation: usize,
}

impl PingPongPixelBuffer {
    /// 创建初始双缓冲区实例。
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        Self {
            buffers: [
                SharedPixelBuffer::new(w, h),
                SharedPixelBuffer::new(w, h),
            ],
            active_idx: 0,
            width: w,
            height: h,
            row_hashes: [Vec::new(), Vec::new()],
            last_cursor: [None, None],
            last_selection: [None, None],
            force_full_redraw: [true, true],
            session_id: String::new(),
            renderer_generation: 0,
        }
    }

    /// 获取当前可原地独占写入的后置缓冲区引用。
    #[inline]
    pub fn get_back_buffer(&mut self) -> &mut SharedPixelBuffer<Rgba8Pixel> {
        &mut self.buffers[self.active_idx]
    }

    /// 提交当前缓冲区生成 Slint Image 交付渲染，并翻转索引指向下一个空闲缓冲区。
    ///
    /// 此时前一个缓冲区的 Slint 视图句柄在新帧到达后即被释放，
    /// 翻转回来的缓冲区引用计数必为 1，确保后续 `make_mut_bytes()` 绝对不发生深拷贝。
    #[inline]
    pub fn commit_to_image(&mut self) -> Image {
        let img = Image::from_rgba8(self.buffers[self.active_idx].clone());
        self.active_idx = 1 - self.active_idx;
        img
    }

    /// 标记双端缓冲区均需要进行全量重绘
    pub fn mark_full_redraw(&mut self) {
        self.force_full_redraw = [true, true];
    }

    /// 设置关联的会话 ID，若会话发生切换则重置脏状态与行指纹
    pub fn set_session_id(&mut self, session_id: &str) {
        if self.session_id != session_id {
            self.session_id = session_id.to_string();
            self.mark_full_redraw();
        }
    }

    /// 比对全局渲染代际 (当字体、字号、调色板变动时代际自增，双缓冲均强制重绘)
    pub fn check_renderer_generation(&mut self, generation: usize) {
        if self.renderer_generation != generation {
            self.renderer_generation = generation;
            self.mark_full_redraw();
        }
    }

    /// 基于当前帧的行哈希指纹、光标与选区计算当前后置缓冲区的行级脏标记掩码。
    pub fn compute_dirty_mask(
        &mut self,
        current_hashes: &[u64],
        current_cursor: Option<(usize, usize, bool)>,
        current_selection: Option<((usize, usize), (usize, usize))>,
    ) -> RowDirtyMask {
        let idx = self.active_idx;
        if self.force_full_redraw[idx] {
            return RowDirtyMask::all();
        }

        let last_hashes = &self.row_hashes[idx];
        if last_hashes.len() != current_hashes.len() {
            return RowDirtyMask::all();
        }

        let mut mask = RowDirtyMask::none();

        // 1. 比对行字符内容/属性哈希指纹
        for (r, (&cur_h, &last_h)) in current_hashes.iter().zip(last_hashes.iter()).enumerate() {
            if cur_h != last_h {
                mask.mark(r);
            }
        }

        // 2. 光标变动行处理 (擦除上一帧光标所在行，并重绘当前帧光标所在行)
        let last_c = self.last_cursor[idx];
        if last_c != current_cursor {
            if let Some((_, r, _)) = last_c {
                mask.mark(r);
            }
            if let Some((_, r, _)) = current_cursor {
                mask.mark(r);
            }
        }

        // 3. 选区变动行处理 (擦除旧选区覆盖行，重绘新选区覆盖行)
        let last_s = self.last_selection[idx];
        if last_s != current_selection {
            if let Some(((c1, r1), (c2, r2))) = last_s {
                let (sr, er) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
                mask.mark_range(sr, er);
                let _ = (c1, c2);
            }
            if let Some(((c1, r1), (c2, r2))) = current_selection {
                let (sr, er) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
                mask.mark_range(sr, er);
                let _ = (c1, c2);
            }
        }

        mask
    }

    /// 在当前后置缓冲区完成像素光栅化后，记录该缓冲区的最新快照状态
    pub fn commit_frame_state(
        &mut self,
        current_hashes: &[u64],
        current_cursor: Option<(usize, usize, bool)>,
        current_selection: Option<((usize, usize), (usize, usize))>,
    ) {
        let idx = self.active_idx;
        self.force_full_redraw[idx] = false;
        self.row_hashes[idx].clear();
        self.row_hashes[idx].extend_from_slice(current_hashes);
        self.last_cursor[idx] = current_cursor;
        self.last_selection[idx] = current_selection;
    }

    /// 尺寸变化时安全对齐双缓冲区。
    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        if self.width != w || self.height != h {
            self.width = w;
            self.height = h;
            self.buffers[0] = SharedPixelBuffer::new(w, h);
            self.buffers[1] = SharedPixelBuffer::new(w, h);
            self.row_hashes[0].clear();
            self.row_hashes[1].clear();
            self.last_cursor = [None, None];
            self.last_selection = [None, None];
            self.force_full_redraw = [true, true];
            self.active_idx = 0;
        }
    }
}
