<script setup lang="ts">
import { ref } from 'vue';
import type { TunnelConfig, TunnelStatusResponse } from '../api';
import { Terminal, Container, Pencil, Trash2, Activity } from 'lucide-vue-next';
import Switch from './ui/Switch.vue';

const props = defineProps<{
  tunnel: TunnelConfig;
  status: TunnelStatusResponse;
}>();

const emit = defineEmits<{
  (e: 'toggle', id: string): void;
  (e: 'edit', tunnel: TunnelConfig): void;
  (e: 'delete', id: string): void;
}>();

const isHovered = ref(false);

const handleToggle = () => {
  emit('toggle', props.tunnel.id);
};

const getStatusClass = (state: string) => {
  if (!state) return 'text-slate-500';
  switch (state.toLowerCase()) {
    case 'running':
      return 'text-green-400';
    case 'starting':
      return 'text-yellow-400';
    case 'stopping':
      return 'text-orange-400';
    case 'error':
      return 'text-red-400';
    default:
      return 'text-slate-500';
  }
};

const getStatusText = (state: string) => {
  if (!state) return 'Unknown';
  if (state.startsWith('error:')) {
    return 'Error';
  }
  return state.charAt(0).toUpperCase() + state.slice(1);
};

const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const sizeIndex = Math.min(i, sizes.length - 1);
  const size = sizes[sizeIndex] ?? 'B';
  return parseFloat((bytes / Math.pow(k, sizeIndex)).toFixed(2)) + size;
};
</script>

<template>
  <div 
    class="group relative flex items-center justify-between rounded-lg bg-slate-800/50 p-4 transition-all hover:bg-slate-800"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <!-- Left: Icon & Info -->
    <div class="flex items-center gap-4">
      <div class="flex h-10 w-10 items-center justify-center rounded-full bg-slate-700 text-slate-300">
        <component :is="tunnel.mode === 'docker' ? Container : Terminal" :size="20" />
      </div>
      <div>
        <h3 class="font-medium text-slate-200">{{ tunnel.name }}</h3>
        <p class="text-xs text-slate-400">
          L:{{ tunnel.local_port }} → 
          <span v-if="tunnel.mode === 'docker'">Docker:{{ tunnel.target_port }}</span>
          <span v-else>{{ tunnel.target_host }}:{{ tunnel.target_port }}</span>
        </p>
      </div>
    </div>

    <!-- Right: Status & Actions -->
    <div class="flex items-center gap-4 relative">
      <!-- Ping/Latency -->
      <div v-if="status.is_running && status.ping !== null" class="flex items-center gap-1.5 text-xs font-medium text-emerald-400">
        <Activity :size="14" />
        <span>{{ status.ping }}ms</span>
      </div>

      <!-- Status indicator based on state -->
      <span class="text-xs font-medium" :class="getStatusClass(status.state || 'unknown')">
        {{ getStatusText(status.state || 'unknown') }}
      </span>

      <!-- Traffic indicators when running -->
      <div v-if="status.is_running" class="flex flex-col text-xs text-slate-400">
        <div class="flex gap-2">
          <span>↑{{ formatBytes(status.send_bytes || 0) }}</span>
          <span>↓{{ formatBytes(status.recv_bytes || 0) }}</span>
        </div>
      </div>

      <!-- Hover Actions -->
      <div
        v-show="isHovered"
        class="flex gap-2 z-10"
      >
        <button
          @click.stop="$emit('edit', tunnel)"
          class="rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-white transition-colors"
          title="Edit"
        >
          <Pencil :size="16" />
        </button>
        <button
          @click.stop="$emit('delete', tunnel.id)"
          class="rounded p-1 text-slate-400 hover:bg-red-900/50 hover:text-red-400 transition-colors"
          title="Delete"
        >
          <Trash2 :size="16" />
        </button>
      </div>

      <!-- Toggle Switch -->
      <Switch :model-value="status.is_running" @update:model-value="handleToggle" />
    </div>
  </div>
</template>
