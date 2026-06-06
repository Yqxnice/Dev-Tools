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
- 写操作（卸载、清理、密码重置、Python 安装等）需要**管理员权限**
- 游客模式仅支持检测和查看

---

## 功能特性

### 🗄️ MySQL

- **版本检测与服务管理**：快速查看已安装的 MySQL 版本及服务状态
- **智能卸载**：多策略卸载（PowerShell / 注册表 / wmic），确保完全移除
- **残留清理**：按实例精准清理，不影响其他版本
- **密码管理**：支持 MySQL 5.6 / 5.7 / 8.0 的密码重置与修改

### 🐍 Python

- **版本检测**：自动扫描本机已安装的 Python 版本
- **安装管理**：查看可用版本列表并一键下载安装
- **虚拟环境**：浏览和管理所有 Python 虚拟环境
- **包管理**：查看已安装包信息
- **镜像源切换**：快速切换 pip 镜像源，加速包安装

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

构建产物为 Windows 安装包（MSI / NSIS）。

---

## 免责声明

本工具不备份数据，所有操作风险由使用者自行承担。请以管理员身份运行，并谨慎使用卸载与清理功能。

---

## 技术栈

- **前端**：Vue 3 + Vite + Pinia
- **桌面框架**：Tauri 2
- **后端**：Rust
