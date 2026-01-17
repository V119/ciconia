import { invoke } from "@tauri-apps/api/core";

export interface TunnelConfig {
  id: string;
  name: string;
  mode: "standard" | "docker";

  // SSH
  ssh_host: string;
  ssh_port: number;
  ssh_username: string;
  auth_type: "password" | "key";
  ssh_password?: string;
  ssh_key_path?: string;

  // Forwarding
  local_port: number;
  target_host: string;
  target_port: number;

  // Docker
  container_name?: string;
  container_port?: String;
}

export interface DockerContainer {
  id: string;
  image: string;
  name: string;
  ports: string[];
  status: string;
}

export interface ContainerDetails {
  ip: string;
}

export interface SshParams {
  host: string;
  port: number;
  username: string;
  auth_type: "password" | "key";
  private_key_path?: string;
  password?: string;
}

export async function getTunnels(): Promise<TunnelConfig[]> {
  return invoke("get_tunnels");
}

export async function saveTunnel(tunnel: TunnelConfig): Promise<void> {
  return invoke("save_tunnel", { tunnel });
}

export async function deleteTunnel(id: string): Promise<void> {
  return invoke("delete_tunnel", { id });
}

export async function startTunnel(id: string): Promise<void> {
  return invoke("start_tunnel", { id });
}

export async function stopTunnel(id: string): Promise<void> {
  return invoke("stop_tunnel", { id });
}

export interface TunnelStatusResponse {
  is_running: boolean;
  ping: number | null;
  state?: string;
  send_bytes?: number;
  recv_bytes?: number;
}

export async function getTunnelStatus(id: string): Promise<TunnelStatusResponse> {
  return invoke("get_tunnel_status", { id });
}

export interface AppSettings {
  launch_at_login: boolean;
  minimize_to_tray_on_close: boolean;
  keep_alive_interval: number;
  default_ssh_key: string | null;
  strict_host_key_checking: boolean;
  connection_timeout: number;
  auto_reconnect: boolean;
  theme: string;
  language: string;
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function fetchContainers(params: SshParams): Promise<DockerContainer[]> {
  return invoke("fetch_containers", { params });
}

export async function getContainerDetails(params: SshParams, containerId: string): Promise<ContainerDetails> {
  return invoke("get_container_details", { params, containerId });
}
