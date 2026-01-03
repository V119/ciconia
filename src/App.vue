<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { Plus } from "lucide-vue-next";
import { 
  getTunnels, 
  saveTunnel as apiSaveTunnel, 
  deleteTunnel as apiDeleteTunnel, 
  startTunnel, 
  stopTunnel, 
  getTunnelStatus,
  type TunnelStatusResponse,
  getSettings,
  type TunnelConfig 
} from "./api";
import { applyTheme, initThemeListener } from "./utils/theme";

import TunnelCard from "./components/TunnelCard.vue";
import TunnelModal from "./components/TunnelModal.vue";
import SettingsModal from "./components/SettingsModal.vue";
import LogsModal from "./components/LogsModal.vue";

// State
const tunnels = ref<TunnelConfig[]>([]);
const tunnelStatuses = ref<Record<string, TunnelStatusResponse>>({});
const showTunnelModal = ref(false);
const editingTunnel = ref<TunnelConfig | null>(null);
const showSettings = ref(false);
const showLogs = ref(false);

// Fetch Data
const refreshTunnels = async () => {
  tunnels.value = await getTunnels();
  checkStatuses();
};

const checkStatuses = async () => {
  for (const t of tunnels.value) {
    tunnelStatuses.value[t.id] = await getTunnelStatus(t.id);
  }
};

// Interval for status check
let statusInterval: number | null = null;

onMounted(async () => {
  // Initialize theme
  try {
    const settings = await getSettings();
    applyTheme(settings.theme as 'system' | 'dark' | 'light');
    initThemeListener(() => settings.theme as 'system' | 'dark' | 'light');
  } catch (e) {
    console.error("Failed to load settings/theme", e);
  }

  await refreshTunnels();
  statusInterval = window.setInterval(checkStatuses, 2000);

  // Listen for tray events
  await listen("open-settings", () => {
    showSettings.value = true;
  });
  await listen("open-logs", () => {
    showLogs.value = true;
  });
});

onUnmounted(() => {
  if (statusInterval) clearInterval(statusInterval);
});

// Actions
const handleAdd = () => {
  editingTunnel.value = null;
  showTunnelModal.value = true;
};

const handleEdit = (tunnel: TunnelConfig) => {
  editingTunnel.value = tunnel;
  showTunnelModal.value = true;
};

const handleDelete = async (id: string) => {
  if (confirm("Are you sure you want to delete this tunnel?")) {
    await apiDeleteTunnel(id);
    await refreshTunnels();
  }
};

const handleToggle = async (id: string) => {
  const isRunning = tunnelStatuses.value[id]?.is_running || false;
  try {
    if (isRunning) {
      await stopTunnel(id);
    } else {
      await startTunnel(id);
    }
  } catch (e) {
    alert(`Error: ${e}`);
  } finally {
    await checkStatuses(); // Immediate check
  }
};

const handleSave = async (data: TunnelConfig) => {
  try {
    await apiSaveTunnel(data);
    showTunnelModal.value = false;
    await refreshTunnels();
  } catch (e) {
    alert(`Failed to save: ${e}`);
  }
};
</script>

<template>
  <div class="min-h-screen bg-slate-950 text-slate-200 selection:bg-blue-500/30 font-sans">
    
    <!-- Header -->
    <header class="sticky top-0 z-10 flex items-center justify-between border-b border-slate-800 bg-slate-950/80 px-6 py-4 backdrop-blur-md">
      <div class="flex items-center gap-2">
        <h1 class="text-lg font-bold tracking-tight text-white">Ciconia</h1>
      </div>
      
      <button 
        @click="handleAdd"
        class="group flex h-8 w-8 items-center justify-center rounded-full bg-slate-800 text-slate-400 transition-all hover:bg-blue-600 hover:text-white hover:shadow-lg hover:shadow-blue-500/20 active:scale-95"
        title="Add Tunnel"
      >
        <Plus :size="18" />
      </button>
    </header>

    <!-- Main Content -->
    <main class="container mx-auto max-w-3xl p-6">
      <div v-if="tunnels.length === 0" class="flex h-64 flex-col items-center justify-center text-slate-500">
        <p class="mb-4 text-sm">No tunnels configured yet.</p>
        <button 
          @click="handleAdd"
          class="rounded-md bg-slate-800 px-4 py-2 text-sm font-medium text-slate-300 hover:bg-slate-700 hover:text-white"
        >
          Create your first tunnel
        </button>
      </div>

      <div v-else class="space-y-3">
        <TunnelCard 
          v-for="tunnel in tunnels" 
          :key="tunnel.id"
          :tunnel="tunnel"
          :status="tunnelStatuses[tunnel.id] || { is_running: false, ping: null }"
          @toggle="handleToggle"
          @edit="handleEdit"
          @delete="handleDelete"
        />
      </div>
    </main>

    <!-- Modals -->
    <TunnelModal 
      :is-open="showTunnelModal"
      :edit-data="editingTunnel"
      @close="showTunnelModal = false"
      @save="handleSave"
    />

    <SettingsModal 
      :is-open="showSettings"
      @close="showSettings = false"
    />

    <LogsModal 
      :is-open="showLogs"
      @close="showLogs = false"
    />
  </div>
</template>

<style>
/* Global scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #334155;
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: #475569;
}
</style>
