//! 系统托盘 (System Tray) 运行服务。
//!
//! 在 Windows 平台下利用原生 Shell_NotifyIcon 与轻量 Win32 消息泵实现托盘常驻，
//! 负责鼠标交互（单/双击恢复、右键菜单呼出）、主窗口状态联动与应用优雅退出。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use slint::ComponentHandle;
use crate::generated::AppWindow;

#[cfg(windows)]
mod win_tray {
    use super::*;
    use slint::winit_030::WinitWindowAccessor;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, POINT};
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
        DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, PostMessageW,
        PostQuitMessage, RegisterClassExW, SetForegroundWindow, TrackPopupMenuEx,
        TranslateMessage, CS_OWNDC, HICON, LR_DEFAULTCOLOR, MF_SEPARATOR, MF_STRING, MSG,
        TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DESTROY, WM_LBUTTONDBLCLK,
        WM_LBUTTONUP, WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
    };

    const WM_TRAYICON: u32 = WM_USER + 101;
    const TRAY_UID: u32 = 1001;

    // 托盘右键菜单命令 ID
    const CMD_SHOW_WINDOW: u32 = 2001;
    const CMD_NEW_SESSION: u32 = 2002;
    const CMD_SETTINGS: u32 = 2003;
    const CMD_EXIT: u32 = 2004;

    pub struct WindowsTrayHandle {
        hwnd: HWND,
        is_running: Arc<AtomicBool>,
        thread_handle: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for WindowsTrayHandle {
        fn drop(&mut self) {
            self.is_running.store(false, Ordering::SeqCst);
            unsafe {
                if !self.hwnd.is_null() {
                    // 移除托盘图标
                    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = self.hwnd;
                    nid.uID = TRAY_UID;
                    Shell_NotifyIconW(NIM_DELETE, &nid);

                    // 通知消息窗口退出
                    PostMessageW(self.hwnd, WM_DESTROY, 0, 0);
                }
            }
            if let Some(th) = self.thread_handle.take() {
                let _ = th.join();
            }
        }
    }

    /// 从内嵌的 PNG 资产解析并创建 HICON 句柄
    pub(crate) unsafe fn load_embedded_tray_icon() -> HICON {
        static TRAY_PNG_BYTES: &[u8] = smagical_ui_view::TRAY_PNG_BYTES;
        unsafe {
            CreateIconFromResourceEx(
                TRAY_PNG_BYTES.as_ptr() as *mut u8,
                TRAY_PNG_BYTES.len() as u32,
                1, // TRUE 表示图标
                0x00030000,
                32,
                32,
                LR_DEFAULTCOLOR,
            )
        }
    }

    /// 启动 Windows 原生托盘后台服务线程
    pub fn start_tray(window_weak: slint::Weak<AppWindow>) -> anyhow::Result<WindowsTrayHandle> {
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_thread = Arc::clone(&is_running);

        let (hwnd_sender, hwnd_receiver) = std::sync::mpsc::channel::<isize>();

        let thread_handle = std::thread::Builder::new()
            .name("smalux-system-tray".to_string())
            .spawn(move || {
                unsafe {
                    // 1. 注册专属消息接收隐藏窗口类
                    let class_name: Vec<u16> = "SmaluxTrayMsgWindowClass\0".encode_utf16().collect();
                    let hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
                    let wnd_class = WNDCLASSEXW {
                        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                        style: CS_OWNDC,
                        lpfnWndProc: Some(tray_wnd_proc),
                        cbClsExtra: 0,
                        cbWndExtra: 0,
                        hInstance: hinstance,
                        hIcon: 0 as _,
                        hCursor: 0 as _,
                        hbrBackground: 0 as _,
                        lpszMenuName: std::ptr::null(),
                        lpszClassName: class_name.as_ptr(),
                        hIconSm: 0 as _,
                    };
                    RegisterClassExW(&wnd_class);

                    // 2. 创建隐藏消息窗口
                    let window_title: Vec<u16> = "SmaluxTrayMessageSink\0".encode_utf16().collect();
                    let hwnd = CreateWindowExW(
                        0,
                        class_name.as_ptr(),
                        window_title.as_ptr(),
                        0,
                        0, 0, 0, 0,
                        0 as _,
                        0 as _,
                        hinstance,
                        std::ptr::null(),
                    );

                    if hwnd.is_null() {
                        let _ = hwnd_sender.send(0);
                        return;
                    }

                    // 3. 加载托盘图标并注册到 Windows 任务栏通知区
                    let hicon = load_embedded_tray_icon();
                    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = hwnd;
                    nid.uID = TRAY_UID;
                    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
                    nid.uCallbackMessage = WM_TRAYICON;
                    nid.hIcon = hicon;

                    // 设置 Tooltip: Smalux SSH - 终端工作台
                    let tip_text: Vec<u16> = "Smalux SSH - 现代终端工作台\0".encode_utf16().collect();
                    let copy_len = tip_text.len().min(nid.szTip.len());
                    nid.szTip[..copy_len].copy_from_slice(&tip_text[..copy_len]);

                    let ok = Shell_NotifyIconW(NIM_ADD, &nid);
                    if ok == 0 {
                        let err = windows_sys::Win32::Foundation::GetLastError();
                        tracing::warn!(target: "smalux::tray", "Shell_NotifyIconW(NIM_ADD) 失败, Win32 错误码: 0x{:08X}", err);
                        DestroyWindow(hwnd);
                        let _ = hwnd_sender.send(0);
                        return;
                    }

                    let _ = hwnd_sender.send(hwnd as isize);

                    // 保存 Weak 窗口指针到全局关联（供窗口过程回调使用）
                    TRAY_WINDOW_WEAK.with(|slot| {
                        *slot.borrow_mut() = Some(window_weak);
                    });

                    // 4. 标准 Win32 消息泵循环
                    let mut msg: MSG = std::mem::zeroed();
                    while is_running_thread.load(Ordering::Relaxed) && GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    // 退出清理
                    Shell_NotifyIconW(NIM_DELETE, &nid);
                    DestroyWindow(hwnd);
                }
            })?;

        let hwnd_raw = hwnd_receiver.recv().unwrap_or(0);
        if hwnd_raw == 0 {
            anyhow::bail!("创建系统托盘消息窗口失败");
        }

        let hwnd = hwnd_raw as HWND;
        tracing::info!(target: "smalux::tray", "Windows 原生系统托盘服务初始化成功 (HWND: {:?})", hwnd);
        Ok(WindowsTrayHandle {
            hwnd,
            is_running,
            thread_handle: Some(thread_handle),
        })
    }

    std::thread_local! {
        static TRAY_WINDOW_WEAK: std::cell::RefCell<Option<slint::Weak<AppWindow>>> = const { std::cell::RefCell::new(None) };
    }

    /// 恢复并激活主窗口到前台
    fn restore_main_window() {
        TRAY_WINDOW_WEAK.with(|slot| {
            if let Some(weak) = slot.borrow().as_ref() {
                let weak_clone = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_clone.upgrade() {
                        let _ = w.show();
                        w.window().set_minimized(false);
                        w.window().with_winit_window(|winit_window| {
                            winit_window.set_visible(true);
                            winit_window.focus_window();
                        });
                    }
                });
            }
        });
    }

    /// 托盘右键菜单弹出
    unsafe fn show_context_menu(hwnd: HWND) {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }

            let item_show: Vec<u16> = "🖥️  显示主窗口\0".encode_utf16().collect();
            let item_session: Vec<u16> = "⚡  新建连接 (New Session)\0".encode_utf16().collect();
            let item_settings: Vec<u16> = "⚙️  应用设置 (Settings)\0".encode_utf16().collect();
            let item_exit: Vec<u16> = "🚪  退出 Smalux\0".encode_utf16().collect();

            AppendMenuW(menu, MF_STRING, CMD_SHOW_WINDOW as usize, item_show.as_ptr());
            AppendMenuW(menu, MF_STRING, CMD_NEW_SESSION as usize, item_session.as_ptr());
            AppendMenuW(menu, MF_STRING, CMD_SETTINGS as usize, item_settings.as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(menu, MF_STRING, CMD_EXIT as usize, item_exit.as_ptr());

            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);

            // Win32 经典做法：在弹出前将消息窗口设为前台，菜单消失后防止失焦驻留
            SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenuEx(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                pt.x,
                pt.y,
                hwnd,
                std::ptr::null(),
            );
            DestroyMenu(menu);

            match cmd as u32 {
                CMD_SHOW_WINDOW => {
                    restore_main_window();
                }
                CMD_NEW_SESSION => {
                    TRAY_WINDOW_WEAK.with(|slot| {
                        if let Some(weak) = slot.borrow().as_ref() {
                            let weak_clone = weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_clone.upgrade() {
                                    let _ = w.show();
                                    w.window().set_minimized(false);
                                    w.window().with_winit_window(|win| {
                                        win.set_visible(true);
                                        win.focus_window();
                                    });
                                    // 打开快捷启动器/新建连接窗口
                                    w.global::<crate::generated::WindowBridge>().set_is_command_palette_open(true);
                                }
                            });
                        }
                    });
                }
                CMD_SETTINGS => {
                    TRAY_WINDOW_WEAK.with(|slot| {
                        if let Some(weak) = slot.borrow().as_ref() {
                            let weak_clone = weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_clone.upgrade() {
                                    let _ = w.show();
                                    w.window().set_minimized(false);
                                    w.window().with_winit_window(|win| {
                                        win.set_visible(true);
                                        win.focus_window();
                                    });
                                    // 切换侧边栏到设置
                                    w.global::<crate::generated::WindowBridge>().set_active_left_tab("settings".into());
                                }
                            });
                        }
                    });
                }
                CMD_EXIT => {
                    TRAY_WINDOW_WEAK.with(|slot| {
                        if let Some(weak) = slot.borrow().as_ref() {
                            let weak_clone = weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_clone.upgrade() {
                                    // 触发强制退出流程
                                    w.invoke_force_close_window();
                                } else {
                                    std::process::exit(0);
                                }
                            });
                        }
                    });
                }
                _ => {}
            }
        }
    }

    /// 窗口过程函数处理托盘回调事件
    unsafe extern "system" fn tray_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WM_TRAYICON => {
                    match lparam as u32 {
                        WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                            restore_main_window();
                            0
                        }
                        WM_RBUTTONUP => {
                            show_context_menu(hwnd);
                            0
                        }
                        _ => 0,
                    }
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    0
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

/// 系统托盘服务控制器
pub struct TrayService {
    #[cfg(windows)]
    _handle: win_tray::WindowsTrayHandle,
}

impl TrayService {
    /// 初始化并启动系统托盘服务
    pub fn init(window: &AppWindow) -> anyhow::Result<Self> {
        #[cfg(windows)]
        {
            let handle = win_tray::start_tray(window.as_weak())?;
            Ok(Self { _handle: handle })
        }
        #[cfg(not(windows))]
        {
            let _ = window;
            Ok(Self {})
        }
    }
}


