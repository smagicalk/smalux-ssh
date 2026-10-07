//! 终端引擎核心层 (Terminal Engine Core)。
//!
//! 包含 PTY 伪终端进程托管、ANSI/VT100 状态机解析与终端会话实例管理。

pub mod backend;
pub mod double_buffer;
pub mod highlight;
pub mod instance;
pub mod key_encoder;
pub mod parser;
pub mod pty;
pub mod pure_ssh;
pub mod renderer;
pub mod split_tree;
pub mod ssh_config;

pub use backend::TerminalBackend;
pub use double_buffer::{PingPongPixelBuffer, RowDirtyMask};
pub use pure_ssh::PureSshProcess;
pub use highlight::{HighlightEngine, SpanHighlight, open_browser_url};

pub use instance::TerminalInstance;
pub use key_encoder::encode_key_event;
pub use parser::TerminalParser;
pub use pty::{PtyProcess, PtySize};
pub use renderer::{TerminalPalette, TerminalRenderer};
pub use split_tree::{
    PaneComputedLayout, PanePixelLayout, SplitNode, SplitOrientation, SplitterComputedLayout,
    SplitterPixelLayout,
};
pub use ssh_config::{KeyTempGuard, SshLaunchConfig};

/// 根据主题深浅模式动态同步更新终端全局调色板与各活跃会话的脏标记重绘
pub fn update_terminal_palette_for_color_mode(
    renderer_opt: &mut Option<TerminalRenderer>,
    active_terminals: &mut std::collections::HashMap<String, TerminalInstance>,
    is_light: bool,
) {
    if let Some(renderer) = renderer_opt {
        let palette = if is_light {
            TerminalPalette::light()
        } else {
            TerminalPalette::dark()
        };
        renderer.update_palette(palette);
    }
    for instance in active_terminals.values_mut() {
        instance.parser.mark_dirty();
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_size_default() {
        let size = PtySize::default();
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
    }

    #[test]
    fn test_alacritty_parser_basic() {
        let mut parser = TerminalParser::new(80, 24);
        assert_eq!(parser.size(), (80, 24));
        assert!(parser.take_dirty());

        parser.process(b"Hello, Smalux Terminal!\r\n");
        let (col, row) = parser.cursor_point();
        assert_eq!(col, 0);
        assert_eq!(row, 1);

        // 测试动态调整尺寸
        parser.resize(120, 40);
        assert_eq!(parser.size(), (120, 40));
    }

    #[test]
    fn test_alacritty_parser_clear() {
        let mut parser = TerminalParser::new(80, 24);
        parser.process(b"Some text on screen");
        parser.clear();
        let (col, row) = parser.cursor_point();
        assert_eq!(col, 0);
        assert_eq!(row, 0);
    }

    #[test]
    fn test_alacritty_parser_scrollback_and_helpers() {
        let mut parser = TerminalParser::new(80, 5);
        // 输出超过 5 行的内容以产生历史回滚
        for i in 1..=20 {
            parser.process(format!("Line #{}\r\n", i).as_bytes());
        }

        let (history_size, initial_offset) = parser.scroll_info();
        assert!(history_size > 0, "历史行数应大于 0");
        assert_eq!(initial_offset, 0, "初始视口偏移量应在底端 0");

        // 向上滚动 5 行
        parser.scroll_delta(5);
        let (_, offset_scrolled) = parser.scroll_info();
        assert!(offset_scrolled > 0, "回滚后偏移量应大于 0");

        // 翻页与回到底部
        parser.scroll_page_up();
        parser.scroll_to_top();
        let (_, top_offset) = parser.scroll_info();
        assert!(top_offset >= offset_scrolled);

        parser.scroll_to_bottom();
        let (_, bot_offset) = parser.scroll_info();
        assert_eq!(bot_offset, 0, "回到底部后偏移量应为 0");
    }

    #[test]
    fn test_alacritty_parser_cjk_unicode() {
        let mut parser = TerminalParser::new(80, 10);
        parser.process("你好，Smalux 终端！✨ 🚀\r\n".as_bytes());
        let (_, row) = parser.cursor_point();
        assert_eq!(row, 1);
    }

    #[test]
    fn test_renderer_initialization_and_rasterization() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器应成功初始化 JetBrains Mono 字体");
        let (cw, ch) = renderer.cell_size();
        assert!(cw >= 5);
        assert!(ch >= 10);
        assert_eq!(renderer.padding_x, 16);
        assert_eq!(renderer.padding_y, 8);

        let mut parser = TerminalParser::new(40, 10);
        parser.process(b"Hello Rust Bitmap Terminal!\r\n");

        let total_w = 40 * cw + renderer.padding_x * 2;
        let total_h = 10 * ch + renderer.padding_y * 2;

        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(total_w, total_h);
        renderer.render_to_buffer(parser.term(), None, &mut pixel_buffer);

        assert_eq!(pixel_buffer.width(), total_w);
        assert_eq!(pixel_buffer.height(), total_h);
    }

    #[test]
    fn test_renderer_ansi_and_truecolor_support() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器应成功初始化");
        let (cw, ch) = renderer.cell_size();
        let mut parser = TerminalParser::new(80, 10);

        // 输出 ANSI 16 色 + 256 色 + 24-bit TrueColor
        parser.process(b"\x1b[31mRed Text \x1b[32mGreen Text \x1b[38;2;255;128;0mTrueColor Orange\x1b[0m\r\n");

        let total_w = 80 * cw + renderer.padding_x * 2;
        let total_h = 10 * ch + renderer.padding_y * 2;
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(total_w, total_h);
        renderer.render_to_buffer(parser.term(), None, &mut pixel_buffer);

        assert_eq!(pixel_buffer.width(), total_w);
        assert_eq!(pixel_buffer.height(), total_h);
    }

    #[test]
    fn test_renderer_cell_styles_flags() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器应成功初始化");
        let (cw, ch) = renderer.cell_size();
        let mut parser = TerminalParser::new(80, 10);

        // 测试加粗、下划线、删除线、反色
        parser.process(b"\x1b[1mBold\x1b[0m \x1b[4mUnderline\x1b[0m \x1b[9mStrike\x1b[0m \x1b[7mInverse\x1b[0m\r\n");

        let total_w = 80 * cw + renderer.padding_x * 2;
        let total_h = 10 * ch + renderer.padding_y * 2;
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(total_w, total_h);
        renderer.render_to_buffer(parser.term(), None, &mut pixel_buffer);

        assert_eq!(pixel_buffer.width(), total_w);
    }

    #[test]
    fn test_renderer_zero_size_buffer_safety() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器初始化成功");
        let parser = TerminalParser::new(80, 24);
        let mut empty_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(0, 0);

        // 不应 panic 或发生越界
        renderer.render_to_buffer(parser.term(), None, &mut empty_buffer);
        assert_eq!(empty_buffer.width(), 0);
    }

    #[test]
    fn test_renderer_scrollback_negative_offset() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器初始化成功");
        let (cw, ch) = renderer.cell_size();
        let mut parser = TerminalParser::new(40, 5);

        for i in 1..=30 {
            parser.process(format!("History line content #{}\r\n", i).as_bytes());
        }

        // 向上滚动 10 行
        parser.scroll_delta(10);

        let total_w = 40 * cw + renderer.padding_x * 2;
        let total_h = 5 * ch + renderer.padding_y * 2;
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(total_w, total_h);
        renderer.render_to_buffer(parser.term(), None, &mut pixel_buffer);

        assert_eq!(pixel_buffer.width(), total_w);
    }

    #[test]
    fn test_key_encoder_all_directions_and_fn() {
        assert_eq!(encode_key_event("\u{F700}", false, false, false), b"\x1b[A"); // Up
        assert_eq!(encode_key_event("\u{F701}", false, false, false), b"\x1b[B"); // Down
        assert_eq!(encode_key_event("\u{F704}", false, false, false), b"\x1bOP"); // F1
        assert_eq!(encode_key_event("\u{F70F}", false, false, false), b"\x1b[24~"); // F12
        assert_eq!(encode_key_event("x", false, false, true), b"\x1bx"); // Alt+x
    }

    #[test]
    fn test_renderer_dynamic_palette_and_font_update() {
        let mut renderer = TerminalRenderer::new(14.0).expect("初始化渲染器");
        assert_eq!(renderer.font_size(), 14.0);

        // 测试动态更新调色板
        let new_palette = TerminalPalette {
            default_bg: [0x00, 0x00, 0x00, 0xff],
            default_fg: [0xff, 0xff, 0xff, 0xff],
            ..Default::default()
        };
        renderer.update_palette(new_palette);
        assert_eq!(renderer.palette().default_bg, [0x00, 0x00, 0x00, 0xff]);


        // 测试动态更新字号
        assert!(renderer.update_font_size(18.0).is_ok());
        assert_eq!(renderer.font_size(), 18.0);
        let (cw_18, ch_18) = renderer.cell_size();
        assert!(cw_18 >= 8);
        assert!(ch_18 >= 18);
    }

    #[test]
    fn test_renderer_opacity_and_padding() {
        let mut renderer = TerminalRenderer::new(14.0).expect("初始化渲染器");

        // 设置半透明壁纸透底
        renderer.set_background_opacity(75);
        assert_eq!(renderer.palette().default_bg[3], 191);

        // 设置自定义内边距
        renderer.set_padding(24, 12);
        assert_eq!(renderer.padding_x, 24);
        assert_eq!(renderer.padding_y, 12);
    }

    #[test]
    fn test_parser_and_renderer_selection_highlight_and_copy() {
        let mut renderer = TerminalRenderer::new(14.0).expect("初始化渲染器");
        let (cw, ch) = renderer.cell_size();
        let mut parser = TerminalParser::new(40, 5);

        parser.process(b"Selectable line of text\r\nSecond line here\r\n");

        // 划选 "Selectable" (列 0..9, 行 0)
        parser.set_selection((0, 0), (9, 0));
        let copied = parser.copy_selection_text();
        assert_eq!(copied, "Selectable");

        // 渲染带有选区的高亮图像
        let total_w = 40 * cw + renderer.padding_x * 2;
        let total_h = 5 * ch + renderer.padding_y * 2;
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(total_w, total_h);
        renderer.render_to_buffer(parser.term(), parser.selection(), &mut pixel_buffer);
        assert_eq!(pixel_buffer.width(), total_w);

        // 清空选区
        parser.clear_selection();
        assert!(parser.selection().is_none());
        assert_eq!(parser.copy_selection_text(), "");
    }

    #[test]
    fn test_row_dirty_mask_operations() {
        let mut mask = RowDirtyMask::none();
        assert!(mask.is_empty());
        assert!(!mask.is_all());
        assert!(!mask.is_dirty(0));
        assert!(!mask.is_dirty(100));

        mask.mark(5);
        assert!(!mask.is_empty());
        assert!(mask.is_dirty(5));
        assert!(!mask.is_dirty(4));
        assert!(!mask.is_dirty(6));

        mask.mark_range(10, 15);
        for r in 10..=15 {
            assert!(mask.is_dirty(r));
        }
        assert!(!mask.is_dirty(9));
        assert!(!mask.is_dirty(16));

        mask.clear();
        assert!(mask.is_empty());

        let all_mask = RowDirtyMask::all();
        assert!(all_mask.is_all());
        assert!(all_mask.is_dirty(0));
        assert!(all_mask.is_dirty(255));
    }

    #[test]
    fn test_ping_pong_dirty_tracking_and_render() {
        let mut renderer = TerminalRenderer::new(14.0).expect("渲染器初始化成功");
        let (cw, ch) = renderer.cell_size();
        let mut parser = TerminalParser::new(40, 10);
        parser.process(b"Initial line content\r\n");

        let total_w = 40 * cw + renderer.padding_x * 2;
        let total_h = 10 * ch + renderer.padding_y * 2;
        let mut ping_pong = PingPongPixelBuffer::new(total_w, total_h);

        // 第 1 帧渲染：初次加载触发全屏重绘
        renderer.render_to_ping_pong(parser.term(), None, &mut ping_pong, "test-sess");
        let _img1 = ping_pong.commit_to_image();

        // 第 2 帧渲染：内容未变动，脏行掩码应为空 (0 脏行)
        let _mask = ping_pong.compute_dirty_mask(&renderer.scratch.row_hashes, None, None);
        // 注：由于双缓冲机制，buf 1 仍处于初次绘制状态，完成 buf 1 绘制后两者均同步
        renderer.render_to_ping_pong(parser.term(), None, &mut ping_pong, "test-sess");
        let _img2 = ping_pong.commit_to_image();

        // 第 3 帧渲染：内容仍未变动，此时两端缓冲均已同步，脏标记应为完全空
        let clean_mask = ping_pong.compute_dirty_mask(&renderer.scratch.row_hashes, None, None);
        assert!(clean_mask.is_empty(), "静态终端画面在双缓冲同步后不应产生脏行");

        // 产生第 2 行输出变动
        parser.process(b"Second line dynamic text\r\n");
        renderer.render_to_ping_pong(parser.term(), None, &mut ping_pong, "test-sess");
        let _img3 = ping_pong.commit_to_image();
    }
}




