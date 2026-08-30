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

impl Default for TerminalPalette {
    fn default() -> Self {
        Self {
            default_bg: [0x1e, 0x1f, 0x22, 0xff],     // Darcula 暗黑背景 #1E1F22
            default_fg: [0xbc, 0xbe, 0xc4, 0xff],     // 柔和白字 #BCBEC4
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
                [0xbc, 0xbe, 0xc4, 0xff], // 7: White
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
const EMBEDDED_JETBRAINS_MONO: &[u8] =
    include_bytes!("../../ui/assets/fonts/JetBrainsMono-Regular.ttf");

/// 终端字符点阵光栅化与像素帧生成渲染器。
pub struct TerminalRenderer {
    /// 字体解析对象
    font: Font,
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
}

impl TerminalRenderer {
    /// 初始化终端位图光栅化渲染器。
    ///
    /// # 参数
    /// - `font_size`: 字体渲染大小 (单位: 像素，推荐 13.0 ~ 15.0)
    pub fn new(font_size: f32) -> Result<Self, String> {
        let font_data = get_terminal_monospace_font()
            .ok_or_else(|| "未检索到可用的等宽字体 (JetBrains Mono / Consolas / Cascadia Mono)".to_string())?;

        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("解析等宽字体文件失败: {:?}", e))?;

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
            font_size,
            cell_width,
            cell_height,
            padding_x: 16,
            padding_y: 8,
            baseline,
            ascii_cache,
            glyph_cache: HashMap::new(),
            palette: TerminalPalette::default(),
        })
    }



    /// 获取单字符单元格网格像素尺寸 `(cell_width, cell_height)`。
    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_width, self.cell_height)
    }

    /// 动态热更新调色板配色方案 (ANSI 16 色、前景色、背景色与光标色)。
    pub fn update_palette(&mut self, palette: TerminalPalette) {
        self.palette = palette;
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
        self.font_size = font_size;
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.baseline = baseline;
        self.ascii_cache = ascii_cache;
        self.glyph_cache.clear();

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

        Ok(())
    }

    /// 动态设置终端底色不透明度百分比 (0 ~ 100)，用于透出下层壁纸。
    pub fn set_background_opacity(&mut self, opacity_pct: u8) {
        let alpha = ((opacity_pct.min(100) as f32 / 100.0) * 255.0).round() as u8;
        self.palette.default_bg[3] = alpha;
    }

    /// 动态设置终端视口内边距留白。
    pub fn set_padding(&mut self, padding_x: u32, padding_y: u32) {
        self.padding_x = padding_x;
        self.padding_y = padding_y;
    }

    /// 获取当前字号。
    pub fn font_size(&self) -> f32 {
        self.font_size
    }


    /// 将 `alacritty_terminal` 的网格内容光栅化渲染至 Slint `SharedPixelBuffer<Rgba8Pixel>`。
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

        // 1. 底色全屏快速填充 (按 32 位整型批量操作，大幅提升高帧率渲染性能)
        let bg_u32 = u32::from_ne_bytes(def_bg);
        let u32_slice: &mut [u32] = unsafe {
            std::slice::from_raw_parts_mut(raw_pixels.as_mut_ptr() as *mut u32, total_bytes / 4)
        };
        u32_slice.fill(bg_u32);


        let display_offset = term.grid().display_offset() as i32;
        let content = term.renderable_content();
        let cursor_point = content.cursor.point;
        let is_cursor_visible = term.mode().contains(TermMode::SHOW_CURSOR) && display_offset == 0;
        let padding_x = self.padding_x;
        let padding_y = self.padding_y;


        // 2. 逐字符单元格遍历光栅化 (精准支持回滚历史负行号转换到视口真实行号)
        for renderable_cell in content.display_iter {
            let col = renderable_cell.point.column.0 as u32;
            let screen_row_i32 = renderable_cell.point.line.0 + display_offset;
            if screen_row_i32 < 0 || screen_row_i32 as u32 >= rows || col >= cols {
                continue;
            }
            let row = screen_row_i32 as u32;

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


            // 处理反色 (Inverse video / 选中或反显)
            if flags.contains(alacritty_terminal::term::cell::Flags::INVERSE) {
                std::mem::swap(&mut fg_rgba, &mut bg_rgba);
            }

            // 处理暗淡 (Dim / Faint 弱化文字)
            if flags.contains(alacritty_terminal::term::cell::Flags::DIM) {
                fg_rgba[0] /= 2;
                fg_rgba[1] /= 2;
                fg_rgba[2] /= 2;
            }

            // 2.1 单元格背景色填充 (若背景不是默认底色)
            if bg_rgba != def_bg {
                let max_x = (cell_x + cell_width).min(img_width);
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

            // 2.2 字符字形点阵光栅化 (非隐藏字符)
            let ch = renderable_cell.c;
            if ch != ' '
                && ch != '\0'
                && !flags.contains(alacritty_terminal::term::cell::Flags::HIDDEN)
            {
                let (metrics, bitmap) = self.get_glyph(ch);
                if metrics.width > 0 && metrics.height > 0 {
                    let gx = cell_x as i32 + metrics.xmin.max(0);
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

                            // 伽马/笔画加黑增强 (Stem Darkening)，消除暗色背景下的发虚感，呈现饱满锐利的字形
                            let alpha = if raw_alpha >= 230 {
                                255
                            } else {
                                ((raw_alpha * (512 - raw_alpha)) / 256).min(255)
                            };

                            let px_offset = row_offset + (px as u32 * 4) as usize;
                            if alpha >= 250 {
                                raw_pixels[px_offset] = fg_rgba[0];
                                raw_pixels[px_offset + 1] = fg_rgba[1];
                                raw_pixels[px_offset + 2] = fg_rgba[2];
                            } else {
                                // Alpha 像素线性插值混合
                                let inv_a = 255 - alpha;
                                raw_pixels[px_offset] = ((fg_rgba[0] as u32 * alpha + raw_pixels[px_offset] as u32 * inv_a) / 255) as u8;
                                raw_pixels[px_offset + 1] = ((fg_rgba[1] as u32 * alpha + raw_pixels[px_offset + 1] as u32 * inv_a) / 255) as u8;
                                raw_pixels[px_offset + 2] = ((fg_rgba[2] as u32 * alpha + raw_pixels[px_offset + 2] as u32 * inv_a) / 255) as u8;
                            }
                        }
                    }
                }
            }

            // 2.3 下划线 (Underline) 绘制
            if flags.contains(alacritty_terminal::term::cell::Flags::UNDERLINE) {
                let line_y = (cell_y as i32 + baseline + 2).min(img_height as i32 - 1);
                if line_y >= 0 {
                    let row_offset = (line_y as u32 * img_width * 4) as usize;
                    let max_x = (cell_x + cell_width).min(img_width);
                    for px in cell_x..max_x {
                        let px_offset = row_offset + (px * 4) as usize;
                        raw_pixels[px_offset] = fg_rgba[0];
                        raw_pixels[px_offset + 1] = fg_rgba[1];
                        raw_pixels[px_offset + 2] = fg_rgba[2];
                    }
                }
            }

            // 2.4 删除线 (Strikeout) 绘制
            if flags.contains(alacritty_terminal::term::cell::Flags::STRIKEOUT) {
                let line_y = (cell_y + cell_height / 2).min(img_height - 1);
                let row_offset = (line_y * img_width * 4) as usize;
                let max_x = (cell_x + cell_width).min(img_width);
                for px in cell_x..max_x {
                    let px_offset = row_offset + (px * 4) as usize;
                    raw_pixels[px_offset] = fg_rgba[0];
                    raw_pixels[px_offset + 1] = fg_rgba[1];
                    raw_pixels[px_offset + 2] = fg_rgba[2];
                }
            }



            // 2.3 光标绘制 (反色/高亮方块，仅在最底端未回滚时显示)
            if is_cursor_visible
                && cursor_point.column.0 == col as usize
                && cursor_point.line.0 == renderable_cell.point.line.0
            {
                let max_x = (cell_x + cell_width).min(img_width);
                let max_y = (cell_y + cell_height).min(img_height);
                for py in cell_y..max_y {
                    let row_offset = (py * img_width * 4) as usize;
                    for px in cell_x..max_x {
                        let px_offset = row_offset + (px * 4) as usize;
                        raw_pixels[px_offset] = ((raw_pixels[px_offset] as u32 + cur_col[0] as u32) / 2) as u8;
                        raw_pixels[px_offset + 1] = ((raw_pixels[px_offset + 1] as u32 + cur_col[1] as u32) / 2) as u8;
                        raw_pixels[px_offset + 2] = ((raw_pixels[px_offset + 2] as u32 + cur_col[2] as u32) / 2) as u8;
                    }
                }
            }
        }
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
                let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
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

/// 在当前操作系统或内置资源中搜寻可用的等宽字体二进制数据。
fn get_terminal_monospace_font() -> Option<Vec<u8>> {
    // 1. 优先使用官方开源 JetBrains Mono 嵌入字体
    if !EMBEDDED_JETBRAINS_MONO.is_empty() {
        return Some(EMBEDDED_JETBRAINS_MONO.to_vec());
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
            return Some(data);
        }
    }
    None
}

