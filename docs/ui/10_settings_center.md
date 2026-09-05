# ⚙️ 偏好设置中心与多端云同步矩阵 (Settings Center & Sync Matrix)

本文档详细阐述 `smalux-ssh` 桌面客户端**全屏偏好设置中心 (SettingsCenterView)** 的视觉规范、八大核心业务分类、多端云同步架构、主密码安全机制以及领域桥接 (`SettingsBridge`) 数据契约。

---

## 一、模块定位与界面架构

偏好设置中心作为客户端的全局控制中枢，支持从左侧活动栏底部图标或快捷键一键全屏展开（`main-view == "settings"`）。界面采用沉浸式两栏响应式布局：
- **左侧分类导航 (220px)**：包含 8 大核心分类导航条目、分类图标、激活状态高亮胶囊指示；
- **右侧滚动内容视口**：宽屏自适应（最大宽度约束 880px 并水平居中，保证大屏阅读舒适度），按分组卡片 (`SettingsSectionCard`) 承载各维度配置项。

```text
+---------------------------------------------------------------------------------------------------------+
| [⚙️ 偏好设置]                                                    [✕ 返回工作台 / ESC]                   |
+-------------------+-------------------------------------------------------------------------------------+
| 📁 常规设置        |  ╭─ 启动与窗口行为 ───────────────────────────────────────────────────────────────╮  |
| 🎨 外观与主题      |  │ 开机自动启动                                                        [ 开关: 关 ] │  |
| 💻 终端排版与特效  |  │ 关闭主窗口时最小化至系统托盘                                        [ 开关: 开 ] │  |
| 🌐 网络与代理      |  │ 关闭多个标签页时二次确认                                            [ 开关: 开 ] │  |
| ☁️ 数据同步与备份  |  ╰─────────────────────────────────────────────────────────────────────────────────╯  |
| 🔐 安全与主密码    |  ╭─ 底层图形渲染诊断 ─────────────────────────────────────────────────────────────╮  |
| ⌨️ 快捷键映射      |  │ 当前图形渲染管线: [winit-skia (GPU)]        [实时帧率: 60 FPS]     [管线基准测试] │  |
| ℹ️ 关于与更新      |  ╰─────────────────────────────────────────────────────────────────────────────────╯  |
+-------------------+-------------------------------------------------------------------------------------+
```

---

## 二、八大核心业务分类规范

### 1. 📁 常规设置 (`general`)
- **窗口与生命周期行为**：
  - 开机自动启动 (`start-on-boot`)；
  - 窗口置顶 (`always-on-top`)；
  - 关闭窗口时行为 (`close-action`: 最小化至托盘 / 直接退出)；
  - 关闭含活跃进程的终端 Tab 时确认 (`confirm-close-active`)；
- **图形渲染管线诊断 (`rendering-diagnostics`)**：
  - 显示底层图形管线（`winit-skia`、`winit-skia-opengl`、`winit-femtovg`）；
  - 实时渲染诊断探针（帧率、单帧绘制耗时、脏矩形命中率）。

### 2. 🎨 外观与主题工坊 (`appearance`)
- **深色 / 浅色模式切换**：支持跟随系统、强制深色、强制浅色；
- **内置主题流式网格 (`ThemeFlowGrid`)**：
  - 预设 15+ 款经典主题卡片预览（ANSI 16 色小方块、背景色预览、作者与版本标签）；
  - 支持“新建自定义主题”虚线卡片，一键唤出 `ThemeEditorModal`；
  - 支持从 Windows Terminal / VS Code JSON 格式一键导入外部配色；
- **动态壁纸与沉浸式毛玻璃 (`wallpaper`)**：
  - **模式切换**：纯色背景、单张壁纸、文件夹多图动态轮播；
  - **轮播策略**：轮播间隔（1分、5分、15分、30分、1小时）、转场动效（淡入淡出、平滑推移）；
  - **壁纸不透明度滑动条**：0% ~ 100% 连续调节，配合弹窗/卡片独立透明度配置。
- **界面字体设置 (`ui-font`)**：
  - 系统已安装字体动态扫描下拉框，支持平滑应用至全软件标签与菜单。

### 3. 💻 终端排版与特效 (`terminal`)
- **字体与排版度量**：终端等宽字体族选择、字号、行高微调；
- **光标定制**：
  - **光标形态**：方块 (`block`)、下划线 (`underline`)、竖线光标 (`bar`)；
  - **光标闪烁**：平滑呼吸闪烁开关；
- **缓冲区与回滚**：回滚行数设置（支持 1,000 ~ 100,000 行），超大内存环形缓冲机制；
- **工业级 CRT 显像管着色器 (`crt-shader`)**：
  - 扫描线模拟、微光晕、球面曲率畸变；
- **关键词高亮规则矩阵 (`KeywordHighlightRule`)**：
  - 正则表达式与纯文本模式切换；
  - 独创带色相环的颜色拾取器 (`ColorWheelPickerModal`)，快速配置报警词（如 `error`、`fatal`、`warning`）的前景与背景高亮。

### 4. 🌐 网络连接与代理中继 (`network`)
- **全局出网代理设置**：
  - 代理模式：直接连接 (`direct`)、全局 HTTP 代理 (`http`)、SOCKS5 代理 (`socks5`)；
  - 代理服务器地址与端口输入框（垂直居中对齐、带聚焦反馈）；
  - 代理认证支持：用户名与加密密码输入；
  - 具备独立“保存代理设置”与“连通性即时测速”按钮；
- **TCP 连接与心跳策略**：
  - 连接超时时长（默认 10s）；
  - Keepalive 保活探测间隔与最大无响应重试次数；
  - TCP_NODELAY（禁用 Nagle 算法，降低打字往返延迟）。

### 5. ☁️ 数据同步与备份迁移 (`sync`)
- **多端云同步矩阵 (Multi-Cloud Sync Matrix)**：
  - **本地快照与归档**：一键导出加密 `.smg` 备份压缩包（支持选择是否包含敏感密码/私钥）；
  - **AWS S3 / 兼容对象存储**：配置 Endpoint、Bucket、AccessKey、SecretKey 实现跨机无缝同步；
  - **WebDAV 网盘同步**：支持坚果云、Nextcloud、群晖 WebDAV 协议；
  - **GitHub Gist 同步**：支持通过 Personal Access Token 将配置作为加密 Private Gist 存储；
  - **预留自建备份服务器接口**：提供标准 RESTful 同步契约；
- **外部资产导入与恢复**：
  - 深度扫描本地 `~/.ssh/config`、`~/.ssh/known_hosts`，一键导入已有主机与分组；
  - 恢复出厂设置（具备二次红字警示弹窗）。

### 6. 🔐 安全与主密码金库 (`security`)
- **主密码管理机制 (Master Password)**：
  - 主密码开启/关闭状态开关；
  - **主密码设置与修改表单**：旧密码验证、新密码复杂度强度指示条（弱/中/强）、确认密码输入框；
  - 开启主密码后，所有持久化文件中的 SSH 口令与私钥 Passphrase 均采用 PBKDF2 + AES-256-GCM 高强度加密；
- **终端会话自动锁定**：
  - 闲置锁定定时器（无操作 5 分钟、15 分钟、30 分钟或永不）；
  - 锁定后模糊遮罩全屏终端，必须输入主密码解锁；
- **数据遮蔽防护**：复制机密信息时剪贴板自动清理定时器（30 秒后自动清空）。

### 7. ⌨️ 快捷键映射矩阵 (`keybindings`)
- 包含命令列表、当前绑定快捷键 (`KeyBadge`)、冲突检测提示；
- 支持双击录制新键位并即时校验冲突。

### 8. ℹ️ 关于与更新诊断 (`about`)
- 客户端版本号、Rust 编译架构、提交哈希；
- 检查新版本更新；
- 故障排查日志导出与开源协议展示。

---

## 三、Slint 领域桥接契约 (`SettingsBridge`)

在全新的模块化架构中，偏好设置彻底脱离 `AppWindow`，由 `SettingsBridge` 单例全局统管：

```slint
// ui/features/settings/settings_bridge.slint
export global SettingsBridge {
    // 分类与模式
    in-out property <string> active-category: "general";
    in-out property <bool> is-dark-mode: true;
    in-out property <string> current-theme-id: "builtin.ui.darcula";
    
    // 常规
    in-out property <bool> setting-start-on-boot: false;
    in-out property <bool> setting-always-on-top: false;
    in-out property <string> setting-close-action: "minimize_to_tray";
    
    // 外观与壁纸
    in-out property <string> wallpaper-mode: "none";
    in-out property <float> wallpaper-opacity: 0.85;
    in-out property <string> ui-font-family: "";
    
    // 终端
    in-out property <string> terminal-font-family: "";
    in-out property <int> terminal-font-size: 14;
    in-out property <string> cursor-style: "block";
    in-out property <bool> cursor-blink: true;
    in-out property <int> scrollback-lines: 10000;
    in-out property <bool> flag-crt-shader: false;
    
    // 网络与代理
    in-out property <string> proxy-mode: "direct";
    in-out property <string> proxy-server: "";
    in-out property <int> connect-timeout-secs: 10;
    
    // 安全
    in-out property <bool> master-password-enabled: false;
    in-out property <int> auto-lock-timeout-mins: 15;
    
    // 操作回调
    callback switch-theme(string);
    callback change-wallpaper(string, string, float);
    callback save-proxy-settings(string, string, string, string);
    callback export-backup-archive(bool);
    callback set-master-password(string, string);
    callback factory-reset();
}
```
