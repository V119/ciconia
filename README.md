# Ciconia (Powered by Tauri v2 & Russh)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.92%2B-orange.svg)
![Crate: russh](https://img.shields.io/badge/Crate-russh-red.svg)
![Crate: russh](https://img.shields.io/badge/Crate-tokio-red.svg)
![Vue 3](https://img.shields.io/badge/Vue-3-green.svg)

> **[🇨🇳 简体中文 (Chinese Version)](README_zh-CN.md)**

A modern, high-performance SSH tunnel management tool built on **Tauri 2**. Powered by **`russh`**, a pure Rust SSH client library, it is designed to solve the pain points of connecting to remote servers and Docker container services during development.

It supports not only traditional port forwarding but also introduces a unique **Docker Dynamic Container Tunnel** feature, allowing direct access to internal container services without opening ports on the server firewall.

## ✨ Core Features

### 1. 🌐 Standard Mode
Classic SSH local port forwarding.
*   **Use Case**: Accessing services bound to `127.0.0.1` on a remote server (e.g., MySQL, Redis) or other machines in the remote intranet.
*   **Workflow**: User configures `Local Port` -> `Remote Host` -> `Remote Port`, enabling direct passthrough via SSH tunnel.

### 2. 🐳 Container Mode (Docker)
Designed specifically for containerized environments to solve the issue of tunnel failure caused by changing container IPs.
*   **Smart Discovery**: Uses `russh` to execute commands on the remote server, listing all running Docker containers and their ports in real-time.
*   **Dynamic Connection**:
    *   **Configuration**: Select the target container by searching for keywords (saves Container ID/Name).
    *   **Runtime Resolution**: Every time a connection is initiated, the backend automatically retrieves the container's current **Internal IP** (Cluster IP/Bridge IP).
    *   **Advantage**: Even if the container restarts and the IP changes, the tunnel configuration remains valid and always points to the correct service.
*   **No Mapping Required**: The target container **does not** need to map ports (via `-p`) on the host. The tunnel goes directly into the container's internal network.

## 🛠️ Tech Architecture

This project utilizes high-performance asynchronous networking components from the Rust ecosystem:

*   **Frontend**: Vue 3 + TypeScript + Tailwind CSS (Provides fluid UI interaction).
*   **Backend (Tauri Host)**: Rust.
*   **SSH Core**: [**`russh`**](https://crates.io/crates/russh) (formerly `thrussh`).
    *   **Embedded Implementation**: Does not rely on the system's local `ssh` command.
    *   **Pure Rust**: Uses `russh` to handle SSH handshakes, key authentication, and channel management.
    *   **High Performance**: Uses `russh`'s `DirectTcpIp` channel for efficient traffic forwarding.
    *   **Remote Execution**: Uses `russh`'s `Session` to execute remote Docker commands.

## 🚀 How It Works (Docker Mode)

1.  **User Action**: Clicks "Connect Tunnel".
2.  **Session Establishment**: The Rust backend establishes an SSH connection to the remote server using `russh`.
3.  **Dynamic IP Resolution**:
    *   The backend executes via SSH Channel: `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' <container_id>`.
    *   Retrieves the real IP of the container within the Docker bridge network (e.g., `172.17.0.5`).
4.  **Tunnel Construction**:
    *   Starts a TCP listener locally (`Local Port`).
    *   When traffic arrives, it opens a `direct-tcpip` channel through the SSH connection pointing to `<Container_IP>:<Container_Port>`.
5.  **Data Transfer**: Traffic is encrypted and transferred between the local machine and the container's internal network.

## 📦 Development & Build

### Prerequisites
*   **Rust**: (Latest Stable recommended)
*   **Node.js**: >= 18
*   **Package Manager**: pnpm / npm / yarn
*   **Build Dependencies**: Please refer to the [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Run in Development

```bash
# 1. Install frontend dependencies
npm install

# 2. Start Tauri dev mode (Starts both frontend and Rust backend)
npm run tauri dev

# 3. Build for Production
npm run tauri build
```

### 📋 Server Requirements

To use this software, the remote server must meet the following:

SSH Service: SSHD enabled with AllowTcpForwarding yes (usually enabled by default).

Docker: Docker installed, and the SSH user must have permission to execute docker commands (e.g., added to the docker group).

### 🤝 Contributing

Contributions are welcome! If you want to improve the russh integration or optimize the frontend experience:

Fork the repository.

Create a feature branch (git checkout -b feature/NewFeature).

Commit your changes (git commit -m 'Add NewFeature').

Push to the branch (git push origin feature/NewFeature).

Open a Pull Request.

###📄 License

MIT License
