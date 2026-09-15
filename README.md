# Dev Tools

<div align="center">

![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=for-the-badge&logo=tauri)
![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D?style=for-the-badge&logo=vue.js)
![Rust](https://img.shields.io/badge/Rust-1.75-DEA584?style=for-the-badge&logo=rust)

Windows 本地开发环境管理面板，基于 Tauri 2 + Vue 3 + Rust 构建。

</div>

---

## 系统要求

- Windows 10 或更高版本
- WebView2 运行时（首次启动自动下载安装）
- **管理员权限**：卸载、清理、密码重置、服务控制等写操作需要提权；非管理员模式下可检测和查看，并支持一键 UAC 重启提权

---

## 功能特性

###  MySQL

- **版本检测与服务管理**：通过 Windows ServiceManager API 查询已安装实例及服务状态
- **智能卸载**：多策略卸载（PowerShell / 注册表 / wmic），按实例精准移除
- **残留清理**：按实例隔离清理，不影响其他版本
- **密码管理**：支持 MySQL 5.6 / 5.7 / 8.0 的密码重置，失败时自动恢复原服务

###  Python

- **版本检测**：扫描本机已安装的 Python 版本，标注来源（system）
- **安装管理**：查看可用版本列表并一键下载（支持暂停/恢复/取消）
- **虚拟环境**：浏览和管理所有 Python 虚拟环境
- **包管理**：查看已安装包信息
- **镜像源切换**：按选定解释器切换 pip 镜像源

### PostgreSQL

- **版本检测与服务管理**：查询已安装实例及服务状态
- **智能卸载与残留清理**：与 MySQL 一致的策略
- **密码管理**：支持 PostgreSQL 密码重置

### Java

- **版本检测**：扫描已安装的 Java 版本，标注厂商和来源
- **Maven 镜像源切换**：快速切换 Maven 镜像

###  Node.js

- **版本检测**：扫描已安装的 Node.js 版本
- **npm 镜像源切换**：快速切换 npm 镜像
- **包管理**：查看全局已安装包

###  JetBrains

- **版本检测**：扫描已安装的 JetBrains 产品
- **残留清理**：清理卸载后的配置和缓存残留

---

## 架构设计

### 插件化后端

每个工具模块实现 `ToolPlugin` trait，通过 `PluginManager` 聚合注册。IPC 调用通过 `HashMap<命令名, handler>` O(1) 路由，避免线性遍历。

```
src-tauri/src/
├── lib.rs              # 应用入口、全局命令、启动兜底
├── plugin.rs           # ToolPlugin trait + PluginManager
├── detector_base.rs    # RuntimeInstance trait + 统一去重
├── service_manager.rs  # windows crate ServiceManager API
├── download_control.rs # 下载任务控制（暂停/恢复/取消）
├── logger.rs           # 日志发送 + 后端环形缓冲
├── access.rs           # 管理员权限检测
├── types.rs            # ts-rs 类型定义（自动导出到前端）
├── mysql/              # MySQL 模块（detector/commands/cleaner/fetcher/plugin）
├── postgresql/         # PostgreSQL 模块
├── python/             # Python 模块
├── java/               # Java 模块
├── node/               # Node.js 模块
└── jetbrains/          # JetBrains 模块
```

### IDE 风格前端

```
src/
├── router/             # createWebHashHistory 扁平路由
├── composables/        # useAppInit / useCloseGuard / useDbTool / useGlobalKeyboard / usePermission
├── components/
│   ├── shell/          # WorkbenchShell（双层 Tab）+ LogDock（可折叠日志面板）
│   ├── shared/         # 通用容器（InstanceWorkbench / DownloadList / ResidueWorkflow）
│   ├── common/         # ErrorBoundary
│   ├── mysql/          # MySQL 功能组件
│   ├── postgresql/
│   ├── python/
│   ├── java/
│   ├── node/
│   └── jetbrains/
├── stores/             # Pinia stores（appStore / taskStore / loggerStore / 各工具 store）
├── services/           # IPC 封装层
├── theme/              # CSS 变量 + prefers-color-scheme 跟随系统
└── types/generated/    # ts-rs 自动生成的 TypeScript 类型
```

### 关键约束

- 路由用 `createWebHashHistory`（适配 Tauri 自定义协议）
- 危险操作（卸载/密码重置）用 `n-modal` 确认，禁用 native `confirm()`
- 下载进度事件双重节流（≥200ms + ≥64KB），成功和失败都发终态事件
- 全局禁用右键菜单、F5/F12/Ctrl+滚轮等浏览器原生行为
- 长任务不设 `globalLoading`，用 store 级 ref 避免页面切换后遮罩卡死

---

## 快速开始

### 开发环境

```bash
npm install
npm run tauri dev
```

### 构建发布

```bash
npm run tauri build
```

构建产物为 Windows 安装包（MSI / NSIS），NSIS 默认 `perMachine` 安装模式。

### 类型生成

后端 Rust 类型通过 `ts-rs` 自动生成到 `src/types/generated/`，修改 `types.rs` 后运行：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 免责声明

本工具不备份数据，所有操作风险由使用者自行承担。请以管理员身份运行，并谨慎使用卸载与清理功能。

---

## 技术栈

- **前端**：Vue 3.5 + Vite + Pinia + Vue Router + Naive UI
- **桌面框架**：Tauri 2
- **后端**：Rust + windows crate + tokio
- **类型桥接**：ts-rs
