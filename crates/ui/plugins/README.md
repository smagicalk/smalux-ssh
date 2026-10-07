# UI 插件矩阵 (`crates/ui/plugins`)

`crates/ui/plugins` 目录汇聚了 **smalux-ssh** 的 8 大页面级业务插件。各插件在结构上高度独立，对应桌面左侧活动栏（Activity Bar）的 8 个核心入口，并通过抽屉、主面板与弹窗多态呈现。

---

## 🧩 插件总览与职责映射

| 插件名称 | 目录路径 | 页面定位 | 核心功能 | 伴生组件 / 抽屉 |
| :--- | :--- | :--- | :--- | :--- |
| **Hosts** | [`hosts/`](hosts/README.md) | 主机资产与终端工作区 | 主机树形分组、快速搜索、一键连接、主机编辑/新增 | AI 伴生助手、Linux 状态探针、Tmux 会话管理、主机抽屉 |
| **Files** | [`files/`](files/README.md) | 双盘文件管理器 | 本地/远程双盘文件浏览、路径导航、Tab 标签页管理 | SFTP 传输抽屉、传输任务队列面板、文件拖拽 |
| **Snippets** | [`snippets/`](snippets/README.md) | 代码片段与快捷指令 | 参数化模板提取、多层分组、终端直接注入执行 | 快速命令抽屉、片段编辑与执行弹窗 |
| **Tunnels** | [`tunnels/`](tunnels/README.md) | 网络隧道与跳板机 | 本地/远端端口转发、动态 SOCKS5 代理、多跳跳板拓扑 | 隧道工具抽屉、拓扑监控、隧道配置弹窗 |
| **Credentials** | [`credentials/`](credentials/README.md) | 凭据保险箱 | 私钥/密码/证书/Agent 存储、公钥指纹解析与复制 | 原生 Ed25519/RSA 密钥对生成弹窗、安全零化 |
| **History** | [`history/`](history/README.md) | 会话审计历史 | 会话时间线、连接状态与退出码追踪、一键重新连接 | 快速历史抽屉、终端历史快照审计弹窗 |
| **Settings** | [`settings/`](settings/README.md) | 系统偏好与外观工坊 | 8 大设置分类、主题实时预览、存储模式切换 | 外观工坊自定义取色器弹窗、数据快照导出导入 |
| **Debug** | [`debug/`](debug/README.md) | 开发者调试控制台 | Tracing 环形日志流、批量主机模拟生成、渲染性能探针 | 快速日志抽屉、状态机监控弹窗 |

---

## 🛠️ 插件开发契约与规范

1. **命名空间与导出规范**：
   每个插件必须在其根目录提供统一入口文件（例如 `hosts.slint`、`files.slint`），并通过 `export { ... }` 导出：
   - 主视图组件（如 `HostsView`、`FilesView`）
   - 伴生抽屉组件（如 `HostsDrawer`、`SftpDrawer`）
   - 业务弹窗（如 `HostEditModal`、`GenerateKeyPairModal`）
2. **样式解耦与零硬编码**：
   所有尺寸间距与调色板均引用 `import { AppTheme } from "../../common/ui/theme.slint";`，绝不写入私有硬编码样式。
3. **Rust 桥接通信规范**：
   所有业务交互通过 `import { XXXBridge } from "../../common/ui/bridges/xxx_bridge.slint";` 触发，严禁跨插件直接耦合内部状态。

---

## 🔗 各插件详细文档

- 🖥️ [Hosts 主机资产插件文档](hosts/README.md)
- 📁 [Files 文件管理插件文档](files/README.md)
- 📜 [Snippets 代码片段插件文档](snippets/README.md)
- 🌐 [Tunnels 网络隧道插件文档](tunnels/README.md)
- 🔐 [Credentials 凭据保险箱插件文档](credentials/README.md)
- 🕒 [History 会话历史插件文档](history/README.md)
- ⚙️ [Settings 偏好设置插件文档](settings/README.md)
- 🛠️ [Debug 调试控制台插件文档](debug/README.md)
