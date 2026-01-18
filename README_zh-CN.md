# Ciconia (Powered by Tauri v2 & russh)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.92%2B-orange.svg)
![Crate: russh](https://img.shields.io/badge/Crate-russh-red.svg)
![Crate: russh](https://img.shields.io/badge/Crate-tokio-red.svg)
![Vue 3](https://img.shields.io/badge/Vue-3-green.svg)

> **[🇬🇧 English Version](README.md)**

一款现代化、高性能的 SSH 隧道管理工具。基于 **Tauri 2** 构建，利用纯 Rust 实现的 SSH 客户端库 **`russh`**，旨在解决开发过程中连接远程服务器及 Docker 容器服务的痛点。

它不仅支持传统的端口转发，更首创了**Docker 动态容器隧道**功能，无需在服务器防火墙开放端口，即可直接访问容器内部服务。

## ✨ 核心功能

### 1. 🌐 标准模式 (Standard Mode)
经典的 SSH 本地端口转发功能。
*   **适用场景**：访问远程服务器上绑定在 `127.0.0.1` 的服务（如 MySQL, Redis）或内网其他机器。
*   **工作流**：用户配置 `Local Port` -> `Remote Host` -> `Remote Port`，通过 SSH 隧道直接透传。

### 2. 🐳 容器模式 (Docker Container Mode)
专为容器化环境设计，解决容器 IP 变动导致隧道失效的问题。
*   **智能发现**：利用 `russh` 在远程服务器执行命令，实时列出所有运行中的 Docker 容器及端口。
*   **动态连接**：
    *   **配置阶段**：用户通过关键字搜索并选择目标容器（软件记录 Container ID）。
    *   **运行时解析**：每次发起连接时，后端会自动通过 SSH 获取该容器当前的**内部 IP 地址**（Cluster IP/Bridge IP）。
    *   **优势**：即使容器重启导致 IP 变化，隧道配置也无需修改，始终指向正确的服务。
*   **无需映射**：目标容器**不需要**在宿主机上使用 `-p` 映射端口，隧道直接通往容器内部网络。

## 🛠️ 技术架构

本项目采用了 Rust 生态中高性能的异步网络组件：

*   **Frontend**: Vue 3 + TypeScript + Tailwind CSS (提供流畅的 UI 交互)。
*   **Backend (Tauri Host)**: Rust。
*   **SSH Core**: [**`russh`**](https://crates.io/crates/russh) (原 `thrussh`)。
    *   **完全嵌入式**：不依赖系统本地的 `ssh` 命令行工具。
    *   **纯 Rust 实现**：使用 `russh` 处理 SSH 握手、密钥认证、Channel 管理。
    *   **高性能转发**：使用 `russh` 的 `DirectTcpIp` 通道实现高效的流量转发。
    *   **远程执行**：使用 `russh` 的 `Session` 执行远程 Docker 命令。

## 🚀 运行原理 (Docker 模式)

1.  **用户操作**：点击“连接隧道”。
2.  **建立会话**：Rust 后端使用 `russh` 建立与远程服务器的 SSH 连接。
3.  **动态解析 IP**：
    *   后端通过 SSH Channel 执行命令：`docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' <container_id>`。
    *   获取容器在 Docker 网桥中的真实 IP（例如 `172.17.0.5`）。
4.  **构建隧道**：
    *   在本地开启 TCP 监听（Local Port）。
    *   当有流量进入时，通过 SSH 连接开启 `direct-tcpip` 通道，指向 `<Container_IP>:<Container_Port>`。
5.  **数据传输**：流量在本地和容器内部网络之间加密传输。

## 📦 开发与构建

### 环境要求
*   **Rust**: (建议最新 Stable 版本)
*   **Node.js**: >= 18
*   **包管理器**: pnpm / npm / yarn
*   **构建依赖**: 请参考 [Tauri v2 环境配置文档](https://v2.tauri.app/start/prerequisites/)

### 启动开发环境

```bash
# 1. 安装前端依赖
npm install

# 2. 启动 Tauri 开发模式 (同时启动前端和 Rust 后端)
npm run tauri dev

构建生产版本
npm run tauri build
```

为了使用本软件，远程服务器需要满足：

SSH 服务：开启 SSHD，并允许 TCP 转发 (AllowTcpForwarding yes，通常默认开启)。

Docker：已安装 Docker，且登录的 SSH 用户有权限执行 docker 命令 (建议将用户加入 docker 用户组，避免使用 root)。

### 🤝 贡献指南 (Contributing)

我们非常欢迎社区贡献！如果您想改进 russh 的集成方式或优化前端体验：

Fork 本仓库。

创建特性分支 (git checkout -b feature/NewFeature)。

提交代码 (git commit -m 'Add NewFeature')。

推送到分支 (git push origin feature/NewFeature)。

提交 Pull Request。

### 📄 许可证

MIT License