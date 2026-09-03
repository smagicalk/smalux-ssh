# 生成英文翻译文件 smagical-ui.po
# 从 template.pot 读取所有 msgid，并通过专业 UI 英文术语词典翻译为英文

$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path "$PSScriptRoot\.."
$PotFile = "$ProjectRoot\crates\smagical-ui\translations\template.pot"
$PoDir = "$ProjectRoot\crates\smagical-ui\translations\en\LC_MESSAGES"
$PoFile = "$PoDir\smagical-ui.po"

if (-not (Test-Path $PoDir)) {
    New-Item -ItemType Directory -Force -Path $PoDir | Out-Null
}

# 专业 UI 术语字典
$dict = @{
    "smalux - 现代 Rust SSH 终端工作台" = "smalux - Modern Rust SSH Terminal Workbench"
    "偏好设置" = "Preferences"
    "偏好设置中心" = "Preferences Center"
    "控制客户端外观、终端排版、网络链路与核心首选项" = "Configure client appearance, terminal engine, network rules, and core preferences"
    "搜索设置项 (如: 字体、主题、语言、快捷键...)" = "Search settings (e.g., font, theme, language, shortcuts...)"
    "返回终端 (ESC)" = "Back to Terminal (ESC)"
    "返回终端" = "Back to Terminal"
    "通用与常规" = "General"
    "通用与窗口交互" = "General & Window Interaction"
    "控制客户端界面语言、启动流程与后台挂起行为" = "Control interface language, startup flow, and background suspend behavior"
    "界面显示语言 (Language)" = "Interface Language"
    "设置客户端主界面展示的默认语言" = "Set the default display language of the client"
    "应用启动时默认视图 (Startup View)" = "Default Startup View"
    "冷启动桌面客户端时自动聚焦的初始页面" = "Initial page automatically focused upon cold startup"
    "中央终端视口" = "Central Terminal Viewport"
    "点击窗口关闭按钮时行为" = "Close Window Action"
    "关闭主窗口时最小化到系统托盘，持续保持后台网络隧道与 SSH 会话不掉线" = "Minimize to system tray on close to keep background tunnels and SSH sessions active"
    "桌面气泡消息通知" = "Desktop Bubble Notifications"
    "在终端长命令执行完毕、文件传输成功或网络隧道自动重连时弹出系统通知" = "Pop up system notifications on command completion, file transfer success, or tunnel reconnection"
    "本地核心数据存储目录" = "Local Core Data Storage Directory"
    "外观与视效" = "Appearance"
    "外观与主题配色" = "Appearance & Themes"
    "个性化定制客户端主视觉、终端 ANSI 调色板与背景毛玻璃壁纸" = "Customize primary visuals, terminal ANSI palettes, and background frosted glass"
    "配色方案主题 (Theme)" = "Color Scheme Theme"
    "选择满足当前光照环境与审美偏好的色彩主题" = "Select a color theme suited to your ambient lighting and aesthetic preference"
    "深色外观模式" = "Dark Appearance Mode"
    "切换深色与浅色基础色调" = "Toggle between dark and light base palettes"
    "背景毛玻璃磨砂模式" = "Background Frosted Acrylic Mode"
    "控制窗口底层亚克力/Mica 毛玻璃磨砂透光特效" = "Control window acrylic/Mica frosted glass blur effect"
    "无" = "None"
    "全局磨砂" = "Global Frosted"
    "仅终端视口" = "Terminal Viewport Only"
    "磨砂透底不透明度" = "Frosted Background Opacity"
    "主题强调高亮色 (Accent Color)" = "Theme Accent Color"
    "终端引擎排版" = "Terminal Engine"
    "终端引擎与排版首选项" = "Terminal Engine & Typography"
    "配置 VT100 字符渲染字模、等宽字体族、光标形态与剪贴板交互行为" = "Configure VT100 glyph rendering, monospace font families, cursor styles, and clipboard actions"
    "等宽字体族 (Font Family)" = "Monospace Font Family"
    "终端网格优先渲染的等宽编程字体" = "Monospace programming font prioritized for terminal grid rendering"
    "渲染字模大小 (Font Size)" = "Glyph Render Font Size"
    "光标形态 (Cursor Style)" = "Cursor Style"
    "块状 (Block)" = "Block"
    "竖线 (Bar)" = "Bar"
    "下划线 (Underline)" = "Underline"
    "光标平滑闪烁" = "Cursor Smooth Blinking"
    "历史滚动行缓冲区 (Scrollback Lines)" = "Scrollback Buffer Lines"
    "控制单个终端标签页保留的历史输出行数上限" = "Maximum retained historical output lines per terminal tab"
    "划词自动复制到剪贴板" = "Copy to Clipboard on Selection"
    "鼠标鼠标选中终端文本时自动将其写入系统剪贴板" = "Automatically copy selected terminal text to system clipboard"
    "鼠标右击快速粘贴" = "Paste Immediately on Right Click"
    "在终端视口点击鼠标右键直接粘贴剪贴板内容" = "Right-click anywhere in terminal viewport to paste clipboard content"
    "粘贴多行命令安全告警" = "Warn on Multi-line Paste"
    "检测到剪贴板包含包含换行符时弹出确认框以防误执行" = "Prompt a confirmation dialog when pasting multiple lines to prevent accidental execution"
    "网络与 SSH" = "Network & SSH"
    "网络传输与 SSH 链路策略" = "Network Transport & SSH Connection Strategy"
    "配置默认连接超时、TCP 心跳保活与自动重连退避算法" = "Configure default timeout, TCP keepalive probes, and reconnection backoff"
    "SSH 连接超时时间" = "SSH Connection Timeout"
    "TCP 心跳保活探测间隔" = "TCP Keepalive Probe Interval"
    "网络异常断线自动重连" = "Auto-reconnect on Disconnection"
    "遭遇网络抖动或异常闪断时自动发起指数退避重试" = "Automatically initiate exponential backoff retries upon network jitter or disconnection"
    "快捷键绑定" = "Shortcuts"
    "快捷键与全局键位映射" = "Keybindings & Global Shortcuts"
    "查看并自定义高频终端操作、多窗格分屏与命令面板的快捷操作" = "View and customize high-frequency terminal shortcuts, pane splits, and command palette"
    "数据备份与迁移" = "Backup & Migration"
    "数据备份、导入与跨工具迁移" = "Data Backup, Import & Tool Migration"
    "导出完整加密备份包、从系统 OpenSSH 与第三方终端工具一键导入资产、管理本地配置历史快照" = "Export encrypted backups, import from OpenSSH/third-party tools, and manage local snapshots"
    "⤓ 导出全量加密备份文件 (.toml / .json)" = "⤓ Export Full Encrypted Backup (.toml / .json)"
    "包含所有主机资产、分组层级树、凭据保管箱、常用代码片段与网络隧道规则" = "Includes all host assets, group hierarchies, credentials vault, snippets, and tunnel rules"
    "⤓ 立即导出备份包" = "⤓ Export Backup Now"
    "📥 一键扫描并导入本机 ~/.ssh/config" = "📥 Scan & Import Local ~/.ssh/config"
    "快速解析本机 OpenSSH 配置文件，自动提取 Host 主机别名、IP 地址、端口与 IdentityFile 密钥" = "Parse local OpenSSH configuration to extract host aliases, IPs, ports, and IdentityFiles"
    "🔍 扫描并预览主机清单" = "🔍 Scan & Preview Hosts"
    "📥 从外部工具一键迁移 (Termius / Xshell)" = "📥 One-Click Migration from External Tools (Termius / Xshell)"
    "无缝导入现有终端软件导出的 JSON/CSV 格式资产备份，平滑迁移" = "Seamlessly import JSON/CSV backups exported from existing terminal clients"
    "📂 选择文件导入..." = "📂 Select File to Import..."
    "🕒 本地最近自动配置快照" = "🕒 Recent Local Configuration Snapshots"
    "应用在每次执行批量导入、大改动或升级前自动保存本地轻量快照，支持随时一键无损回滚" = "Automatically creates lightweight snapshots before batch operations, enabling lossless rollback"
    "↺ 回滚" = "↺ Rollback"
    "安全与高级诊断" = "Security & Diagnostics"
    "安全隐私与开发者诊断" = "Security, Privacy & Diagnostics"
    "本地凭据加密主密码、F12 开发者调试控制台入口与运行时诊断日志级别" = "Credential master passphrase encryption, F12 developer console, and diagnostic logs"
    "本地凭据主密码保护 (AES-256)" = "Local Credential Master Passphrase (AES-256)"
    "为本地保存的 SSH 私钥和口令凭据启用主密码加密保护" = "Protect stored SSH private keys and passwords with AES-256 master passphrase"
    "🛠️ 开发者调试控制台 (Developer Debug Panel)" = "🛠️ Developer Debug Console"
    "启用内置的开发者调试工作台抽屉（可按 F12 随时唤起/隐藏）" = "Enable developer debug console drawer (toggle anytime via F12)"
    "危险区：恢复出厂设置" = "Danger Zone: Reset to Factory Defaults"
    "重置所有个性化偏好设置为默认值（不会删除已保存的主机资产与凭据保管箱数据）" = "Reset all preferences to defaults (will not delete host assets or credentials)"
    "⚠️ 恢复默认偏好" = "⚠️ Reset to Defaults"
    "主机资产调试" = "Hosts Debug"
    "凭据密钥调试" = "Credentials Debug"
    "代码片段调试" = "Snippets Debug"
    "网络隧道调试" = "Tunnels Debug"
    "批量数据注入" = "Batch Injection"
    "日志与追踪" = "Logs & Tracing"
    "树形巡检诊断" = "Tree Inspector"
    "渲染性能诊断" = "Performance"
    "配置预设管理" = "Presets"
    "偏好状态同步" = "Settings Sync"
    "新建主机分组" = "New Host Group"
    "所属上级分组:" = "Parent Group:"
    "当前: 顶级根目录" = "Current: Root Directory"
    "当前: 子分组" = "Current: Subgroup"
    "选取上级" = "Select Parent"
    "新分组名称:" = "New Group Name:"
    "请输入分组名称，例如: 微服务集群、测试数据库..." = "Enter group name, e.g. Microservices, Test DB..."
    "取消" = "Cancel"
    "确认创建" = "Create"
    "确认" = "Confirm"
    "新建代码片段文件夹" = "New Snippet Folder"
    "文件夹名称" = "Folder Name"
    "如：Docker 容器运维、K8s 排障..." = "e.g. Docker Ops, K8s Troubleshooting..."
    "上级文件夹 (可选多层嵌套)" = "Parent Folder (Optional nesting)"
    "创建" = "Create"
    "新建网络与隧道规则" = "New Network & Tunnel Rule"
    "请选择您要创建的资产类型，将为您打开专属配置表单" = "Select the rule type to open its configuration form"
    "端口转发 (Port Forwarding)" = "Port Forwarding"
    "本地、远端或动态 SOCKS5 端口转发与穿透" = "Local, remote, or dynamic SOCKS5 port forwarding"
    "跳板机 (JumpHost / Bastion)" = "Jump Host (Bastion)"
    "配置 ProxyJump 堡垒机中继，供主机连接时绑定" = "Configure ProxyJump bastion relay for host connections"
    "出网代理池 (Proxy Server)" = "Outbound Proxy Server"
    "配置 HTTP / SOCKS5 代理节点，供主机统一出网" = "Configure HTTP / SOCKS5 proxy server for unified egress"
    "开发者调试工作台 (Workbench)" = "Developer Debug Workbench"
    "主机数: " = "Hosts: "
    " | 可见节点: " = " | Visible Nodes: "
    "网络与隧道中心" = "Network & Tunnels"
    "凭据管理" = "Credentials"
    "代码片段" = "Snippets"
    "历史记录" = "History"
    "主机" = "Hosts"
    "终端" = "Terminal"
    "设置" = "Settings"
    "新建会话" = "New Session"
    "连接主机" = "Connect Host"
    "断开连接" = "Disconnect"
    "重新连接" = "Reconnect"
    "复制" = "Copy"
    "粘贴" = "Paste"
    "清屏" = "Clear Screen"
    "全屏" = "Full Screen"
    "分屏" = "Split Screen"
    "水平拆分" = "Split Horizontally"
    "垂直拆分" = "Split Vertically"
    "关闭窗格" = "Close Pane"
    "关闭标签页" = "Close Tab"
    "关闭其他" = "Close Others"
    "复制会话" = "Duplicate Session"
    "搜索" = "Search"
    "保存" = "Save"
    "编辑" = "Edit"
    "删除" = "Delete"
    "重命名" = "Rename"
    "刷新" = "Refresh"
    "状态" = "Status"
    "地址" = "Address"
    "端口" = "Port"
    "用户名" = "Username"
    "密码" = "Password"
    "私钥" = "Private Key"
    "备注" = "Notes"
    "在线" = "Online"
    "离线" = "Offline"
    "错误" = "Error"
    "警告" = "Warning"
    "连接中" = "Connecting"
    "已连接" = "Connected"
    "未连接" = "Disconnected"
    "主机资产" = "Host Assets"
    "凭据保管箱" = "Credentials Vault"
    "代码片段库" = "Code Snippets"
    "网络与隧道" = "Network Tunnels"
    "操作历史" = "History"
    "文件管理器" = "File Explorer"
    "新建主机" = "New Host"
    "新建分组" = "New Group"
    "主机名称" = "Host Name"
    "主机地址" = "Host Address"
    "连接端口" = "Port"
    "关联凭据" = "Credential"
    "无凭据 (手动输入)" = "No Credential (Manual)"
    "快速连接" = "Quick Connect"
    "新建凭据" = "New Credential"
    "私钥路径" = "Key Path"
    "选择私钥文件" = "Browse Key File..."
    "代码片段树形视图" = "Snippets Tree"
    "新建片段" = "New Snippet"
    "脚本内容" = "Script Content"
    "运行代码片段" = "Run Snippet"
    "执行" = "Execute"
    "规则名称" = "Rule Name"
    "本地端口" = "Local Port"
    "远端主机" = "Remote Host"
    "远端端口" = "Remote Port"
    "启动规则" = "Start Rule"
    "停止规则" = "Stop Rule"
    "简体中文" = "Simplified Chinese"
    "英文 (English)" = "English"
}

Write-Host "Reading $PotFile..."
# 加载外部 TSV 术语字典表 (若存在)
$tsvPath = "$ProjectRoot\crates\smagical-ui\translations\en_dict.tsv"
if (Test-Path $tsvPath) {
    Get-Content $tsvPath -Encoding UTF8 | ForEach-Object {
        $parts = $_ -split "`t", 2
        if ($parts.Length -ge 2 -and -not [string]::IsNullOrWhiteSpace($parts[0])) {
            $key = $parts[0]
            $val = $parts[1]
            $dict[$key] = $val
            if (-not $dict.ContainsKey($key.Trim())) {
                $dict[$key.Trim()] = $val.Trim()
            }
        }
    }
}

$lines = Get-Content -Path $PotFile -Encoding UTF8

$outputLines = [System.Collections.Generic.List[string]]::new()

$currentMsgid = ""
$inHeader = $true

for ($i = 0; $i -lt $lines.Length; $i++) {
    $line = $lines[$i]

    if ($line.StartsWith('msgid ""') -and $i -eq 2) {
        # PO 文件头部设置
        $outputLines.Add('msgid ""')
        $outputLines.Add('msgstr ""')
        $outputLines.Add('"Project-Id-Version: smalux-ssh 0.1.0\n"')
        $outputLines.Add('"Report-Msgid-Bugs-To: dev@smalux.io\n"')
        $outputLines.Add('"POT-Creation-Date: 2026-09-02 11:30+0000\n"')
        $outputLines.Add('"PO-Revision-Date: 2026-09-02 11:30+0000\n"')
        $outputLines.Add('"Last-Translator: smalux <dev@smalux.io>\n"')
        $outputLines.Add('"Language-Team: English <en@li.org>\n"')
        $outputLines.Add('"Language: en\n"')
        $outputLines.Add('"MIME-Version: 1.0\n"')
        $outputLines.Add('"Content-Type: text/plain; charset=UTF-8\n"')
        $outputLines.Add('"Content-Transfer-Encoding: 8bit\n"')
        $outputLines.Add('"Plural-Forms: nplurals=2; plural=(n != 1);\n"')
        
        # 跳过 pot 原始头部
        while ($i -lt $lines.Length -and $lines[$i] -ne "") {
            $i++
        }
        $outputLines.Add("")
        continue
    }

    if ($line.StartsWith('msgid "')) {
        $raw = $line.Substring(7, $line.Length - 8)
        $currentMsgid = $raw
        $outputLines.Add($line)
        continue
    }

    if ($line.StartsWith('msgstr "')) {
        $trans = $null
        if ($dict.ContainsKey($currentMsgid)) {
            $trans = $dict[$currentMsgid]
        } elseif ($dict.ContainsKey($currentMsgid.Trim())) {
            # 智能保留原文本首尾空格
            $trimmedTrans = $dict[$currentMsgid.Trim()]
            $lead = ""
            if ($currentMsgid -match '^(\s+)') { $lead = $Matches[1] }
            $trail = ""
            if ($currentMsgid -match '(\s+)$') { $trail = $Matches[1] }
            $trans = $lead + $trimmedTrans + $trail
        } else {
            $trans = $currentMsgid
        }

        $transEsc = $trans.Replace('"', '\"')
        $outputLines.Add("msgstr `"$transEsc`"")
        continue
    }

    $outputLines.Add($line)
}

[System.IO.File]::WriteAllLines($PoFile, $outputLines, [System.Text.Encoding]::UTF8)
Write-Host "Successfully generated English PO file: $PoFile"
Write-Host "Total entries processed: $(($outputLines | Where-Object { $_.StartsWith('msgid ') }).Count)"
