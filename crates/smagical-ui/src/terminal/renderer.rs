//! 终端字符网格像素光栅化与位图渲染引擎。
//!
//! 将 `alacritty_terminal` 的二维字符矩阵转换为 `SharedPixelBuffer<Rgba8Pixel>` 位图，支持 24-bit TrueColor、ANSI 256 色与字形缓存。

use std::collections::HashMap;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use fontdue::{Font, FontSettings, Metrics};
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::terminal::parser::TerminalEventListener;


/// 终端字符单元格标准色彩调色板。
#[derive(Clone, Copy, Debug)]
pub struct TerminalPalette {
    /// 默认背景色 RGBA
    pub default_bg: [u8; 4],
    /// 默认前景色 RGBA
    pub default_fg: [u8; 4],
    /// 光标高亮色 RGBA
    pub cursor_color: [u8; 4],
    /// 选区高亮背景色 RGBA
    pub selection_bg: [u8; 4],
    /// ANSI 基础 16 色定义 [黑, 红, 绿, 黄, 蓝, 紫, 青, 白, 亮黑, 亮红, 亮绿, 亮黄, 亮蓝, 亮紫, 亮青, 亮白]
    pub ansi_colors: [[u8; 4]; 16],
}

impl TerminalPalette {
    /// 默认暗黑调色板 (Darcula 主题)
    pub fn dark() -> Self {
        Self::default()
    }

    /// 浅色高对比度调色板 (GitHub Light 主题)
    pub fn light() -> Self {
        Self {
            default_bg: [0xf6, 0xf8, 0xfa, 0xff],     // 浅灰白底 #F6F8FA
            default_fg: [0x24, 0x29, 0x2f, 0xff],     // 深色清晰黑字 #24292F
            cursor_color: [0x09, 0x69, 0xda, 0xff],   // 亮蓝光标 #0969DA
            selection_bg: [0xdd, 0xf4, 0xff, 0xff],   // 浅蓝高亮选区 #DDF4FF
            ansi_colors: [
                [0x24, 0x29, 0x2f, 0xff], // 0: Black
                [0xcf, 0x22, 0x2e, 0xff], // 1: Red
                [0x1a, 0x7f, 0x37, 0xff], // 2: Green
                [0x9a, 0x67, 0x00, 0xff], // 3: Yellow
                [0x09, 0x69, 0xda, 0xff], // 4: Blue
                [0x82, 0x50, 0xdf, 0xff], // 5: Magenta
                [0x1b, 0x7c, 0x83, 0xff], // 6: Cyan
                [0x6e, 0x77, 0x81, 0xff], // 7: White
                [0x57, 0x60, 0x6a, 0xff], // 8: Bright Black
                [0xa4, 0x0e, 0x26, 0xff], // 9: Bright Red
                [0x11, 0x63, 0x29, 0xff], // 10: Bright Green
                [0x7d, 0x4e, 0x00, 0xff], // 11: Bright Yellow
                [0x05, 0x50, 0xae, 0xff], // 12: Bright Blue
                [0x66, 0x39, 0xba, 0xff], // 13: Bright Magenta
                [0x13, 0x5e, 0x65, 0xff], // 14: Bright Cyan
                [0x24, 0x29, 0x2f, 0xff], // 15: Bright White
            ],
        }
    }
}

impl Default for TerminalPalette {
    fn default() -> Self {
        Self {
            default_bg: [0x1e, 0x1f, 0x22, 0xff],     // Darcula 暗黑背景 #1E1F22
            default_fg: [0xf0, 0xf2, 0xf5, 0xff],     // 清晰高亮白字 #F0F2F5 (对齐现代 IDE 终端与界面高对比度)
            cursor_color: [0x35, 0x74, 0xf0, 0xff],   // 亮蓝光标 #3574F0
            selection_bg: [0x21, 0x42, 0x83, 0xff],   // 选区蓝底 #214283
            ansi_colors: [
                [0x1e, 0x1f, 0x22, 0xff], // 0: Black
                [0xf7, 0x54, 0x64, 0xff], // 1: Red
                [0x57, 0xb6, 0x78, 0xff], // 2: Green
                [0xe5, 0xb5, 0x67, 0xff], // 3: Yellow
                [0x35, 0x74, 0xf0, 0xff], // 4: Blue
                [0xc7, 0x7d, 0xb4, 0xff], // 5: Magenta
                [0x00, 0xaa, 0xbe, 0xff], // 6: Cyan
                [0xe4, 0xe6, 0xeb, 0xff], // 7: White
                [0x70, 0x72, 0x78, 0xff], // 8: Bright Black
                [0xff, 0x6b, 0x7a, 0xff], // 9: Bright Red
                [0x6f, 0xc9, 0x8f, 0xff], // 10: Bright Green
                [0xff, 0xc7, 0x77, 0xff], // 11: Bright Yellow
                [0x59, 0x8e, 0xff, 0xff], // 12: Bright Blue
                [0xdc, 0x94, 0xc9, 0xff], // 13: Bright Magenta
                [0x2a, 0xc3, 0xd5, 0xff], // 14: Bright Cyan
                [0xff, 0xff, 0xff, 0xff], // 15: Bright White
            ],
        }
    }
}

/// 内置官方开源 JetBrains Mono 等宽字体二进制数据 (OFL 许可)
const EMBEDDED_JETBRAINS_MONO: &[u8] = crate::JETBRAINS_MONO_BYTES;

/// 单字符单元格渲染紧凑结构体，用于单帧刮板池复用 (0 堆内存开销)
#[derive(Clone, Copy)]
pub struct RenderCell {
    /// 视口列索引
    pub col: u32,
    /// 视口行索引
    pub screen_row: u32,
    /// Alacritty 网格物理行号 (支持负数历史回滚行)
    pub line_i32: i32,
    /// 字符内容
    pub c: char,
    /// 前景色彩定义
    pub fg: AnsiColor,
    /// 背景色彩定义
    pub bg: AnsiColor,
    /// 单元格样式标志位 (宽字符、反色、粗体、下划线等)
    pub flags: alacritty_terminal::term::cell::Flags,
}

/// 预分配单帧渲染刮板池，确保 60FPS 渲染热循环 0 堆内存分配
#[derive(Default)]
pub struct RenderScratch {
    /// 预分配复用的单元格收集向量
    pub display_cells: Vec<RenderCell>,
    /// 预分配按行分布的字符坐标缓存 (按行索引 0..rows)
    pub row_chars: Vec<Vec<(usize, char)>>,
    /// 预分配行高亮匹配结果表
    pub row_highlights: HashMap<u32, Vec<crate::terminal::highlight::SpanHighlight>>,
    /// 预分配单行字符串拼装缓冲区
    pub line_str: String,
    /// 预分配每行哈希指纹缓存 (按行索引 0..rows)
    pub row_hashes: Vec<u64>,
}

impl RenderScratch {
    /// 重置刮板池各字段，维持已有底层内存容量
    pub fn reset(&mut self, rows: usize) {
        self.display_cells.clear();
        self.row_highlights.clear();
        self.line_str.clear();

        if self.row_chars.len() < rows {
            self.row_chars.resize_with(rows, Vec::new);
        }
        for r in 0..rows {
            self.row_chars[r].clear();
        }

        if self.row_hashes.len() < rows {
            self.row_hashes.resize(rows, 0);
        } else {
            self.row_hashes.truncate(rows);
            self.row_hashes.fill(0);
        }
    }
}

/// 终端字符点阵光栅化与像素帧生成渲染器。
pub struct TerminalRenderer {
    /// 字体解析对象
    font: Font,
    /// 系统 CJK 回退字体 (用于中文/日文/韩文等多语言字形光栅化)
    fallback_font: Option<Font>,
    /// 字体点号大小 (默认 14.0 px)
    font_size: f32,
    /// 字符单元格像素宽度 (例如 8 px)
    cell_width: u32,
    /// 字符单元格像素高度 (例如 17 px)
    cell_height: u32,
    /// 终端左右内边距像素 (宽裕留白 16 px)
    pub padding_x: u32,
    /// 终端上下内边距像素 (舒适留白 8 px)
    pub padding_y: u32,
    /// 基线相对单元格顶部的像素偏移量
    baseline: i32,
    /// 常用 ASCII 字符快速扁平数组缓存 (0..128，O(1) 无哈希瞬时寻址)
    ascii_cache: Vec<Option<(Metrics, Vec<u8>)>>,
    /// 扩展 Unicode / CJK 字形光栅化点阵内存缓存 `(char -> (Metrics, Vec<u8>))`
    glyph_cache: HashMap<char, (Metrics, Vec<u8>)>,
    /// 配色方案
    palette: TerminalPalette,
    /// 光标样式 ("block" | "beam" | "underline")
    pub cursor_style: String,
    /// 光标呼吸闪烁开关
    pub cursor_blink: bool,
    /// 终端关键词、URL 与 IPv4 语法高亮规则引擎
    pub highlight_engine: crate::terminal::highlight::HighlightEngine,
    /// 预分配热循环单帧渲染刮板池 (0 堆分配)
    pub scratch: RenderScratch,
    /// 全局渲染代际计数器 (调色板/字体等全局参数变更时自增)
    pub generation: usize,
}

impl TerminalRenderer {
    /// 初始化终端位图光栅化渲染器。
    ///
    /// # 参数
    /// - `font_size`: 字体渲染大小 (单位: 像素，推荐 13.0 ~ 15.0)
    pub fn new(font_size: f32) -> Result<Self, String> {
        let font_data = get_terminal_monospace_font()
            .ok_or_else(|| "未检索到可用的等宽字体 (JetBrains Mono / Consolas / Cascadia Mono)".to_string())?;

        let font = Font::from_bytes(font_data.as_ref(), FontSettings::default())
            .map_err(|e| format!("解析等宽字体文件失败: {:?}", e))?;

        let fallback_font = get_system_cjk_font().and_then(|data| {
            Font::from_bytes(data, FontSettings::default()).ok()
        });

        // 基于基准字符 'M' 与空行度量计算标准等宽网格单元格尺寸
        let m_metrics = font.metrics('M', font_size);
        let cell_width = (m_metrics.advance_width.ceil() as u32).max(7);

        let line_metrics = font.horizontal_line_metrics(font_size);
        let cell_height = if let Some(lm) = line_metrics {
            (lm.new_line_size.ceil() as u32).max(14)
        } else {
            (font_size * 1.25).ceil() as u32
        };

        let baseline = if let Some(lm) = line_metrics {
            lm.ascent.ceil() as i32
        } else {
            (font_size * 0.9) as i32
        };


        let mut ascii_cache = vec![None; 128];
        // 启动时预热光栅化所有常用 ASCII 可见字符 (0x20..=0x7E)
        for b in 0x20u8..=0x7Eu8 {
            let ch = b as char;
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            ascii_cache[b as usize] = Some((metrics, bitmap));
        }

        Ok(Self {
            font,
            fallback_font,
            font_size,
            cell_width,
            cell_height,
            padding_x: 16,
            padding_y: 8,
            baseline,
            ascii_cache,
            glyph_cache: HashMap::new(),
            palette: TerminalPalette::default(),
            cursor_style: "block".to_string(),
            cursor_blink: true,
            highlight_engine: crate::terminal::highlight::HighlightEngine::default(),
            scratch: RenderScratch::default(),
            generation: 0,
        })
    }

    /// 动态热更新终端语法与运维关键词高亮规则
    pub fn update_highlight_rules<I>(&mut self, rules_iter: I)
    where
        I: IntoIterator<Item = (String, String, [u8; 4], bool)>,
    {
        self.highlight_engine.update_from_rules(rules_iter);
    }

    /// 设置光标形态 ("block" | "beam" | "underline")
    pub fn set_cursor_style(&mut self, style: &str) {
        self.cursor_style = style.to_string();
    }

    /// 设置光标是否闪烁
    pub fn set_cursor_blink(&mut self, blink: bool) {
        self.cursor_blink = blink;
    }



    /// 获取单字符单元格网格像素尺寸 `(cell_width, cell_height)`。
    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_width, self.cell_height)
    }

    /// 动态热更新调色板配色方案 (ANSI 16 色、前景色、背景色与光标色)。
    pub fn update_palette(&mut self, mut palette: TerminalPalette) {
        palette.default_bg[3] = self.palette.default_bg[3];
        self.palette = palette;
        self.generation = self.generation.wrapping_add(1);
    }

    /// 获取当前终端调色板快照。
    pub fn palette(&self) -> TerminalPalette {
        self.palette
    }

    /// 动态热更换渲染字体与字号。
    ///
    /// # 参数
    /// - `font_data`: 新字体文件二进制字节流 (TTF/OTF)
    /// - `font_size`: 新字号大小 (像素)
    pub fn update_font(&mut self, font_data: &[u8], font_size: f32) -> Result<(), String> {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("解析字体文件失败: {:?}", e))?;

        let m_metrics = font.metrics('M', font_size);
        let cell_width = (m_metrics.advance_width.ceil() as u32).max(7);

        let line_metrics = font.horizontal_line_metrics(font_size);
        let cell_height = if let Some(lm) = line_metrics {
            (lm.new_line_size.ceil() as u32).max(14)
        } else {
            (font_size * 1.25).ceil() as u32
        };

        let baseline = if let Some(lm) = line_metrics {
            lm.ascent.ceil() as i32
        } else {
            (font_size * 0.9) as i32
        };

        let mut ascii_cache = vec![None; 128];
        for b in 0x20u8..=0x7Eu8 {
            let ch = b as char;
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            ascii_cache[b as usize] = Some((metrics, bitmap));
        }

        self.font = font;
        if self.fallback_font.is_none() {
            self.fallback_font = get_system_cjk_font().and_then(|data| {
                Font::from_bytes(data, FontSettings::default()).ok()
            });
        }
        self.font_size = font_size;
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.baseline = baseline;
        self.ascii_cache = ascii_cache;
        self.glyph_cache.clear();
        self.generation = self.generation.wrapping_add(1);

        Ok(())
    }

    /// 动态更新当前字体的渲染字号 (平滑重缩放网格)。
    pub fn update_font_size(&mut self, font_size: f32) -> Result<(), String> {
        let m_metrics = self.font.metrics('M', font_size);
        let cell_width = (m_metrics.advance_width.ceil() as u32).max(7);

        let line_metrics = self.font.horizontal_line_metrics(font_size);
        let cell_height = if let Some(lm) = line_metrics {
            (lm.new_line_size.ceil() as u32).max(14)
        } else {
            (font_size * 1.25).ceil() as u32
        };

        let baseline = if let Some(lm) = line_metrics {
            lm.ascent.ceil() as i32
        } else {
            (font_size * 0.9) as i32
        };

        let mut ascii_cache = vec![None; 128];
        for b in 0x20u8..=0x7Eu8 {
            let ch = b as char;
            let (metrics, bitmap) = self.font.rasterize(ch, font_size);
            ascii_cache[b as usize] = Some((metrics, bitmap));
        }

        self.font_size = font_size;
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.baseline = baseline;
        self.ascii_cache = ascii_cache;
        self.glyph_cache.clear();
        self.generation = self.generation.wrapping_add(1);

        Ok(())
    }

    /// 动态设置终端底色不透明度百分比 (0 ~ 100)，用于透出下层壁纸。
    pub fn set_background_opacity(&mut self, opacity_pct: u8) {
        let alpha = ((opacity_pct.min(100) as f32 / 100.0) * 255.0).round() as u8;
        self.palette.default_bg[3] = alpha;
        self.generation = self.generation.wrapping_add(1);
    }

    /// 动态设置终端视口内边距留白。
    pub fn set_padding(&mut self, padding_x: u32, padding_y: u32) {
        self.padding_x = padding_x;
        self.padding_y = padding_y;
        self.generation = self.generation.wrapping_add(1);
    }

    /// 获取当前字号。
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
}

/// 将 ANSI 色彩快速转换为 64 位整型指纹 (用于微秒级行哈希计算)
#[inline(always)]
fn ansi_color_to_u64(color: AnsiColor) -> u64 {
    match color {
        AnsiColor::Named(named) => named as u64,
        AnsiColor::Spec(rgb) => ((rgb.r as u64) << 16) | ((rgb.g as u64) << 8) | (rgb.b as u64),
        AnsiColor::Indexed(idx) => (idx as u64) | 0x1000000,
    }
}

impl TerminalRenderer {
    /// 基于 Ping-Pong 双缓冲与行级脏追踪的高效光栅化。
    /// 仅对实际发生内容变动、光标移动或选区覆盖的脏行执行局部重绘，
    /// 其余未变动行完全跳过背景擦除与字形光栅化开销。
    pub fn render_to_ping_pong(
        &mut self,
        term: &alacritty_terminal::Term<TerminalEventListener>,
        selection: Option<((usize, usize), (usize, usize))>,
        ping_pong: &mut crate::terminal::double_buffer::PingPongPixelBuffer,
        session_id: &str,
    ) {
        ping_pong.set_session_id(session_id);
        ping_pong.check_renderer_generation(self.generation);

        let cols = term.columns() as u32;
        let rows = term.screen_lines() as u32;
        let display_offset = term.grid().display_offset() as i32;
        let content = term.renderable_content();
        let cursor_point = content.cursor.point;
        let is_cursor_visible = term.mode().contains(TermMode::SHOW_CURSOR) && display_offset == 0;
        let is_blink_visible = !self.cursor_blink
            || ((std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() / 530) % 2 == 0);

        let current_cursor = if is_cursor_visible {
            let cur_row_i32 = cursor_point.line.0 + display_offset;
            if cur_row_i32 >= 0 && (cur_row_i32 as u32) < rows {
                Some((cursor_point.column.0, cur_row_i32 as usize, is_blink_visible))
            } else {
                None
            }
        } else {
            None
        };

        self.scratch.reset(rows as usize);
        for cell in content.display_iter {
            let col = cell.point.column.0 as u32;
            let screen_row_i32 = cell.point.line.0 + display_offset;
            if screen_row_i32 >= 0 && (screen_row_i32 as u32) < rows && col < cols {
                let r = screen_row_i32 as usize;
                self.scratch.row_chars[r].push((col as usize, cell.c));
                self.scratch.display_cells.push(RenderCell {
                    col,
                    screen_row: screen_row_i32 as u32,
                    line_i32: cell.point.line.0,
                    c: cell.c,
                    fg: cell.fg,
                    bg: cell.bg,
                    flags: cell.flags,
                });

                let cell_hash = (col as u64)
                    ^ ((cell.c as u64) << 8)
                    ^ ((cell.flags.bits() as u64) << 32)
                    ^ (ansi_color_to_u64(cell.fg) << 40)
                    ^ (ansi_color_to_u64(cell.bg) << 48);
                self.scratch.row_hashes[r] = self.scratch.row_hashes[r].rotate_left(5) ^ cell_hash;
            }
        }

        let dirty_mask = ping_pong.compute_dirty_mask(
            &self.scratch.row_hashes,
            current_cursor,
            selection,
        );

        if dirty_mask.is_empty() {
            return;
        }

        let pixel_buffer = ping_pong.get_back_buffer();
        self.render_internal_with_collected(
            term,
            selection,
            pixel_buffer,
            dirty_mask,
            cursor_point,
            is_cursor_visible,
            is_blink_visible,
        );

        ping_pong.commit_frame_state(
            &self.scratch.row_hashes,
            current_cursor,
            selection,
        );
    }

    /// 将 `alacritty_terminal` 的网格内容全量光栅化渲染至 Slint `SharedPixelBuffer<Rgba8Pixel>`。
    ///
    /// # 参数
    /// - `term`: Alacritty 终端状态机实例
    /// - `selection`: 鼠标划选的高亮选区范围 (可选)
    /// - `pixel_buffer`: 目标像素缓冲区 (尺寸需与 `cols * cell_width, rows * cell_height` 匹配)
    pub fn render_to_buffer(
        &mut self,
        term: &alacritty_terminal::Term<TerminalEventListener>,
        selection: Option<((usize, usize), (usize, usize))>,
        pixel_buffer: &mut SharedPixelBuffer<Rgba8Pixel>,
    ) {
        let cols = term.columns() as u32;
        let rows = term.screen_lines() as u32;
        let display_offset = term.grid().display_offset() as i32;
        let content = term.renderable_content();
        let cursor_point = content.cursor.point;
        let is_cursor_visible = term.mode().contains(TermMode::SHOW_CURSOR) && display_offset == 0;
        let is_blink_visible = !self.cursor_blink
            || ((std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() / 530) % 2 == 0);

        self.scratch.reset(rows as usize);
        for cell in content.display_iter {
            let col = cell.point.column.0 as u32;
            let screen_row_i32 = cell.point.line.0 + display_offset;
            if screen_row_i32 >= 0 && (screen_row_i32 as u32) < rows && col < cols {
                let r = screen_row_i32 as usize;
                self.scratch.row_chars[r].push((col as usize, cell.c));
                self.scratch.display_cells.push(RenderCell {
                    col,
                    screen_row: screen_row_i32 as u32,
                    line_i32: cell.point.line.0,
                    c: cell.c,
                    fg: cell.fg,
                    bg: cell.bg,
                    flags: cell.flags,
                });
            }
        }

        self.render_internal_with_collected(
            term,
            selection,
            pixel_buffer,
            crate::terminal::double_buffer::RowDirtyMask::all(),
            cursor_point,
            is_cursor_visible,
            is_blink_visible,
        );
    }

    /// 内部核心光栅化引擎：根据行级脏标记掩码选择性光栅化特定行
    fn render_internal_with_collected(
        &mut self,
        _term: &alacritty_terminal::Term<TerminalEventListener>,
        selection: Option<((usize, usize), (usize, usize))>,
        pixel_buffer: &mut SharedPixelBuffer<Rgba8Pixel>,
        dirty_mask: crate::terminal::double_buffer::RowDirtyMask,
        cursor_point: alacritty_terminal::index::Point,
        is_cursor_visible: bool,
        is_blink_visible: bool,
    ) {
        if dirty_mask.is_empty() {
            return;
        }

        let img_width = pixel_buffer.width();
        let img_height = pixel_buffer.height();
        if img_width == 0 || img_height == 0 {
            return;
        }

        let raw_pixels = pixel_buffer.make_mut_bytes();
        let total_bytes = (img_width * img_height * 4) as usize;
        if raw_pixels.len() < total_bytes {
            return;
        }

        let cell_width = self.cell_width;
        let cell_height = self.cell_height;
        let baseline = self.baseline;
        let def_bg = self.palette.default_bg;
        let def_fg = self.palette.default_fg;
        let cur_col = self.palette.cursor_color;
        let padding_x = self.padding_x;
        let padding_y = self.padding_y;

        let bg_u32 = u32::from_ne_bytes(def_bg);
        let u32_slice: &mut [u32] = unsafe {
            std::slice::from_raw_parts_mut(raw_pixels.as_mut_ptr() as *mut u32, total_bytes / 4)
        };

        // 1. 底色填充：全量脏或局部行脏填充 (按 32 位整型批量操作)
        if dirty_mask.is_all() {
            u32_slice.fill(bg_u32);
        } else {
            let w = img_width as usize;
            for r in 0..256 {
                if dirty_mask.is_dirty(r) {
                    let y_start = (padding_y + r as u32 * cell_height) as usize;
                    if y_start >= img_height as usize {
                        break;
                    }
                    let y_end = (y_start + cell_height as usize).min(img_height as usize);
                    for y in y_start..y_end {
                        let start = y * w;
                        let end = (start + w).min(u32_slice.len());
                        u32_slice[start..end].fill(bg_u32);
                    }
                }
            }
        }

        // 2. 仅对脏行匹配高亮规则
        let rows_len = self.scratch.row_chars.len();
        let mut row_highlights = std::mem::take(&mut self.scratch.row_highlights);
        for row in 0..rows_len {
            if !dirty_mask.is_dirty(row) {
                continue;
            }
            let chars = &mut self.scratch.row_chars[row];
            if chars.is_empty() {
                continue;
            }
            chars.sort_unstable_by_key(|(c, _)| *c);
            self.scratch.line_str.clear();
            let mut last_col = 0;
            for &(c, ch) in chars.iter() {
                while last_col < c {
                    self.scratch.line_str.push(' ');
                    last_col += 1;
                }
                self.scratch.line_str.push(if ch != '\0' { ch } else { ' ' });
                last_col += 1;
            }
            let spans = self.highlight_engine.match_line(&self.scratch.line_str);
            if !spans.is_empty() {
                row_highlights.insert(row as u32, spans);
            }
        }

        // 3. 逐字符单元格遍历光栅化 (仅光栅化脏行)
        let display_cells = std::mem::take(&mut self.scratch.display_cells);
        for renderable_cell in &display_cells {
            let row = renderable_cell.screen_row;
            if !dirty_mask.is_dirty(row as usize) {
                continue;
            }
            let col = renderable_cell.col;

            let cell_x = col * cell_width + padding_x;
            let cell_y = row * cell_height + padding_y;

            if cell_x >= img_width || cell_y >= img_height {
                continue;
            }

            // 判断当前单元格是否落在鼠标划选的高亮选区中
            let is_selected = if let Some(((c1, r1), (c2, r2))) = selection {
                let ((s_r, s_c), (e_r, e_c)) = if r1 < r2 || (r1 == r2 && c1 <= c2) {
                    ((r1, c1), (r2, c2))
                } else {
                    ((r2, c2), (r1, c1))
                };
                let r = row as usize;
                let c = col as usize;
                if r < s_r || r > e_r {
                    false
                } else if s_r == e_r {
                    c >= s_c && c <= e_c
                } else if r == s_r {
                    c >= s_c
                } else if r == e_r {
                    c <= e_c
                } else {
                    true
                }
            } else {
                false
            };

            let flags = renderable_cell.flags;
            if flags.contains(alacritty_terminal::term::cell::Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            let is_wide = flags.contains(alacritty_terminal::term::cell::Flags::WIDE_CHAR);
            let slot_width = if is_wide { cell_width * 2 } else { cell_width };

            let mut fg_rgba = if is_selected {
                [0xff, 0xff, 0xff, 0xff]
            } else {
                self.resolve_color(renderable_cell.fg, &def_fg)
            };
            let mut bg_rgba = if is_selected {
                self.palette.selection_bg
            } else {
                self.resolve_color(renderable_cell.bg, &def_bg)
            };

            // 检查当前单元格是否命中关键词、URL 或 IP 语法高亮规则 (选区优先)
            let mut is_rule_underline = false;
            if !is_selected {
                if let Some(spans) = row_highlights.get(&row) {
                    for s in spans {
                        if (col as usize) >= s.start_col && (col as usize) < s.end_col {
                            fg_rgba = s.color;
                            if s.underline {
                                is_rule_underline = true;
                            }
                            break;
                        }
                    }
                }
            }

            // 处理反色
            if flags.contains(alacritty_terminal::term::cell::Flags::INVERSE) {
                std::mem::swap(&mut fg_rgba, &mut bg_rgba);
            }

            // 处理暗淡
            if flags.contains(alacritty_terminal::term::cell::Flags::DIM) {
                fg_rgba[0] /= 2;
                fg_rgba[1] /= 2;
                fg_rgba[2] /= 2;
            }

            // 3.1 单元格背景色填充
            if bg_rgba != def_bg {
                let max_x = (cell_x + slot_width).min(img_width);
                let max_y = (cell_y + cell_height).min(img_height);
                for py in cell_y..max_y {
                    let row_offset = (py * img_width * 4) as usize;
                    for px in cell_x..max_x {
                        let px_offset = row_offset + (px * 4) as usize;
                        raw_pixels[px_offset] = bg_rgba[0];
                        raw_pixels[px_offset + 1] = bg_rgba[1];
                        raw_pixels[px_offset + 2] = bg_rgba[2];
                        raw_pixels[px_offset + 3] = bg_rgba[3];
                    }
                }
            }

            // 3.2 字符字形点阵光栅化
            let ch = renderable_cell.c;
            if ch != ' '
                && ch != '\0'
                && !flags.contains(alacritty_terminal::term::cell::Flags::HIDDEN)
            {
                let (metrics, bitmap) = self.get_glyph(ch);
                if metrics.width > 0 && metrics.height > 0 {
                    let gx = if is_wide {
                        if (metrics.width as u32) < slot_width {
                            cell_x as i32 + ((slot_width - metrics.width as u32) / 2) as i32
                        } else {
                            cell_x as i32 + metrics.xmin.max(0)
                        }
                    } else {
                        cell_x as i32 + metrics.xmin.max(0)
                    };
                    let gy = cell_y as i32 + baseline - metrics.ymin - metrics.height as i32;

                    for by in 0..metrics.height {
                        let py = gy + by as i32;
                        if py < 0 || py >= img_height as i32 {
                            continue;
                        }

                        let row_offset = (py as u32 * img_width * 4) as usize;
                        let b_row_offset = by * metrics.width;

                        for bx in 0..metrics.width {
                            let px = gx + bx as i32;
                            if px < 0 || px >= img_width as i32 {
                                continue;
                            }

                            let raw_alpha = bitmap[b_row_offset + bx] as u32;
                            if raw_alpha == 0 {
                                continue;
                            }

                            let alpha = if raw_alpha >= 180 {
                                255
                            } else {
                                (raw_alpha * 255 / 180).min(255)
                            };

                            let px_offset = row_offset + (px as u32 * 4) as usize;
                            if alpha >= 240 {
                                raw_pixels[px_offset] = fg_rgba[0];
                                raw_pixels[px_offset + 1] = fg_rgba[1];
                                raw_pixels[px_offset + 2] = fg_rgba[2];
                            } else {
                                let inv_a = 255 - alpha;
                                raw_pixels[px_offset] = ((fg_rgba[0] as u32 * alpha + raw_pixels[px_offset] as u32 * inv_a) / 255) as u8;
                                raw_pixels[px_offset + 1] = ((fg_rgba[1] as u32 * alpha + raw_pixels[px_offset + 1] as u32 * inv_a) / 255) as u8;
                                raw_pixels[px_offset + 2] = ((fg_rgba[2] as u32 * alpha + raw_pixels[px_offset + 2] as u32 * inv_a) / 255) as u8;
                            }
                        }
                    }
                }
            }

            // 3.3 下划线绘制
            if flags.contains(alacritty_terminal::term::cell::Flags::UNDERLINE) || is_rule_underline {
                let line_y = (cell_y as i32 + baseline + 2).min(img_height as i32 - 1);
                if line_y >= 0 {
                    let row_offset = (line_y as u32 * img_width * 4) as usize;
                    let max_x = (cell_x + slot_width).min(img_width);
                    for px in cell_x..max_x {
                        let px_offset = row_offset + (px * 4) as usize;
                        raw_pixels[px_offset] = fg_rgba[0];
                        raw_pixels[px_offset + 1] = fg_rgba[1];
                        raw_pixels[px_offset + 2] = fg_rgba[2];
                    }
                }
            }

            // 3.4 删除线绘制
            if flags.contains(alacritty_terminal::term::cell::Flags::STRIKEOUT) {
                let line_y = (cell_y + cell_height / 2).min(img_height - 1);
                let row_offset = (line_y * img_width * 4) as usize;
                let max_x = (cell_x + slot_width).min(img_width);
                for px in cell_x..max_x {
                    let px_offset = row_offset + (px * 4) as usize;
                    raw_pixels[px_offset] = fg_rgba[0];
                    raw_pixels[px_offset + 1] = fg_rgba[1];
                    raw_pixels[px_offset + 2] = fg_rgba[2];
                }
            }

            // 3.5 光标绘制
            if is_cursor_visible
                && is_blink_visible
                && cursor_point.column.0 == col as usize
                && cursor_point.line.0 == renderable_cell.line_i32
            {
                let cur_w = if is_wide { slot_width } else { cell_width };
                let (c_min_x, c_max_x, c_min_y, c_max_y) = match self.cursor_style.as_str() {
                    "beam" => (cell_x, (cell_x + 2).min(img_width), cell_y, (cell_y + cell_height).min(img_height)),
                    "underline" => (cell_x, (cell_x + cur_w).min(img_width), (cell_y + cell_height.saturating_sub(2)).min(img_height), (cell_y + cell_height).min(img_height)),
                    _ => (cell_x, (cell_x + cur_w).min(img_width), cell_y, (cell_y + cell_height).min(img_height)),
                };

                for py in c_min_y..c_max_y {
                    let row_offset = (py * img_width * 4) as usize;
                    for px in c_min_x..c_max_x {
                        let px_offset = row_offset + (px * 4) as usize;
                        raw_pixels[px_offset] = ((raw_pixels[px_offset] as u32 + cur_col[0] as u32) / 2) as u8;
                        raw_pixels[px_offset + 1] = ((raw_pixels[px_offset + 1] as u32 + cur_col[1] as u32) / 2) as u8;
                        raw_pixels[px_offset + 2] = ((raw_pixels[px_offset + 2] as u32 + cur_col[2] as u32) / 2) as u8;
                    }
                }
            }
        }

        // 将刮板缓冲区所有权归还，供下一帧光栅化复用容量
        self.scratch.display_cells = display_cells;
        self.scratch.row_highlights = row_highlights;
    }

    /// 高速检索或按需光栅化字符点阵，优先使用 O(1) 扁平 ASCII 数组。
    #[inline(always)]
    fn get_glyph(&mut self, ch: char) -> &(Metrics, Vec<u8>) {
        let code = ch as usize;
        if code < 128 {
            if self.ascii_cache[code].is_none() {
                let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
                self.ascii_cache[code] = Some((metrics, bitmap));
            }
            self.ascii_cache[code].as_ref().unwrap()
        } else {
            if !self.glyph_cache.contains_key(&ch) {
                // 1,024 槽上限保护与轻量 GC：避免长时高频日志/多语言/Emoji 输出导致显存点阵无界膨胀
                if self.glyph_cache.len() >= 1024 {
                    self.glyph_cache.clear();
                }
                let (metrics, bitmap) = if self.font.lookup_glyph_index(ch) != 0 {
                    self.font.rasterize(ch, self.font_size)
                } else if let Some(fb) = &self.fallback_font {
                    if fb.lookup_glyph_index(ch) != 0 {
                        fb.rasterize(ch, self.font_size)
                    } else {
                        self.font.rasterize(ch, self.font_size)
                    }
                } else {
                    self.font.rasterize(ch, self.font_size)
                };
                self.glyph_cache.insert(ch, (metrics, bitmap));
            }
            &self.glyph_cache[&ch]
        }
    }

    /// 将 `alacritty_terminal` 的颜色枚举解析为 RGBA8 色值。
    fn resolve_color(&self, color: AnsiColor, default: &[u8; 4]) -> [u8; 4] {
        match color {
            AnsiColor::Named(named) => match named {
                NamedColor::Black => self.palette.ansi_colors[0],
                NamedColor::Red => self.palette.ansi_colors[1],
                NamedColor::Green => self.palette.ansi_colors[2],
                NamedColor::Yellow => self.palette.ansi_colors[3],
                NamedColor::Blue => self.palette.ansi_colors[4],
                NamedColor::Magenta => self.palette.ansi_colors[5],
                NamedColor::Cyan => self.palette.ansi_colors[6],
                NamedColor::White => self.palette.ansi_colors[7],
                NamedColor::BrightBlack => self.palette.ansi_colors[8],
                NamedColor::BrightRed => self.palette.ansi_colors[9],
                NamedColor::BrightGreen => self.palette.ansi_colors[10],
                NamedColor::BrightYellow => self.palette.ansi_colors[11],
                NamedColor::BrightBlue => self.palette.ansi_colors[12],
                NamedColor::BrightMagenta => self.palette.ansi_colors[13],
                NamedColor::BrightCyan => self.palette.ansi_colors[14],
                NamedColor::BrightWhite => self.palette.ansi_colors[15],
                NamedColor::Foreground => self.palette.default_fg,
                NamedColor::Background => self.palette.default_bg,
                NamedColor::Cursor => self.palette.cursor_color,
                _ => *default,
            },
            AnsiColor::Spec(rgb) => [rgb.r, rgb.g, rgb.b, 255],
            AnsiColor::Indexed(idx) => {
                if idx < 16 {
                    self.palette.ansi_colors[idx as usize]
                } else if idx < 232 {
                    // 6x6x6 颜色立方体
                    let mut i = idx - 16;
                    let b = (i % 6) * 51;
                    i /= 6;
                    let g = (i % 6) * 51;
                    let r = (i / 6) * 51;
                    [r, g, b, 255]
                } else {
                    // 24 阶灰度
                    let gray = (idx - 232) * 10 + 8;
                    [gray, gray, gray, 255]
                }
            }
        }
    }
}

/// 在当前操作系统或内置资源中搜寻可用的等宽字体二进制数据 (零拷贝共享，消除 400KB+ 堆分配)。
fn get_terminal_monospace_font() -> Option<std::borrow::Cow<'static, [u8]>> {
    // 1. 优先使用官方开源 JetBrains Mono 嵌入字体 (直接提供静态内存切片引用)
    if !EMBEDDED_JETBRAINS_MONO.is_empty() {
        return Some(std::borrow::Cow::Borrowed(EMBEDDED_JETBRAINS_MONO));
    }

    #[cfg(windows)]
    let candidate_paths = [
        "C:\\Windows\\Fonts\\consola.ttf",
        "C:\\Windows\\Fonts\\CascadiaMono.ttf",
        "C:\\Windows\\Fonts\\cour.ttf",
        "C:\\Windows\\Fonts\\lucon.ttf",
    ];

    #[cfg(target_os = "macos")]
    let candidate_paths = [
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/SFMono-Regular.otf",
    ];

    #[cfg(target_os = "linux")]
    let candidate_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    ];

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    let candidate_paths: [&str; 0] = [];

    for path in &candidate_paths {
        if let Ok(data) = std::fs::read(path) {
            return Some(std::borrow::Cow::Owned(data));
        }
    }
    None
}

/// 在当前操作系统中检索可用的 CJK 中文字体二进制数据 (用于等宽终端中文/全角字形回退)
pub fn get_system_cjk_font() -> Option<Vec<u8>> {
    #[cfg(windows)]
    let candidate_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\Deng.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];

    #[cfg(target_os = "macos")]
    let candidate_paths = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Songti.ttc",
    ];

    #[cfg(target_os = "linux")]
    let candidate_paths = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    ];

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    let candidate_paths: [&str; 0] = [];

    for path in &candidate_paths {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

/// 根据字体名称或族名在系统字体目录中查找匹配的字体文件数据
pub fn find_font_by_name(font_name: &str) -> Option<Vec<u8>> {
    let fn_clean = font_name.trim();
    if fn_clean.is_empty() || fn_clean == "JetBrains Mono" {
        if !EMBEDDED_JETBRAINS_MONO.is_empty() {
            return Some(EMBEDDED_JETBRAINS_MONO.to_vec());
        }
    }

    let fn_lower = fn_clean.to_lowercase();

    #[cfg(windows)]
    {
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let fonts_dir = std::path::Path::new(&win_dir).join("Fonts");

        let candidate_files: Vec<&str> = match fn_lower.as_str() {
            "consolas" => vec!["consola.ttf", "consolab.ttf"],
            "cascadia code" | "cascadia mono" => vec!["CascadiaCode.ttf", "CascadiaMono.ttf", "Cascadia.ttf"],
            "courier new" => vec!["cour.ttf", "courbd.ttf"],
            "fira code" => vec!["FiraCode-Regular.ttf", "FiraCode.ttf"],
            "source code pro" => vec!["SourceCodePro-Regular.ttf"],
            "hack" => vec!["Hack-Regular.ttf"],
            "meslolgs nf" | "meslo" => vec!["MesloLGS NF Regular.ttf"],
            _ => vec![],
        };

        for fname in candidate_files {
            let p = fonts_dir.join(fname);
            if let Ok(bytes) = std::fs::read(&p) {
                return Some(bytes);
            }
        }

        // 也检查用户字体目录: %LOCALAPPDATA%\Microsoft\Windows\Fonts
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let user_fonts = std::path::Path::new(&local_app_data).join("Microsoft").join("Windows").join("Fonts");
            if let Ok(entries) = std::fs::read_dir(user_fonts) {
                let target_compact = fn_lower.replace(' ', "");
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains(&target_compact) && (name.ends_with(".ttf") || name.ends_with(".otf")) {
                        if let Ok(bytes) = std::fs::read(entry.path()) {
                            return Some(bytes);
                        }
                    }
                }
            }
        }
    }

    // Fallback: 如果传的是文件完整路径
    if std::path::Path::new(font_name).is_file() {
        return std::fs::read(font_name).ok();
    }

    None
}


