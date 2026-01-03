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
    <div class="flex items-center gap-4">
      <div v-if="status.is_running && status.ping !== null" class="flex items-center gap-1.5 text-xs font-medium text-emerald-400">
        <Activity :size="14" />
        <span>{{ status.ping }}ms</span>
      </div>
      
      <span class="text-xs font-medium" :class="status.is_running ? 'text-green-400' : 'text-slate-500'">
        {{ status.is_running ? 'Active' : 'Off' }}
      </span>
      
      <!-- Toggle Switch -->
      <Switch :model-value="status.is_running" @update:model-value="handleToggle" />

      <!-- Hover Actions (Absolute or pushed) -->
      <div 
        class="absolute right-16 flex gap-2 transition-opacity duration-200"
        :class="isHovered ? 'opacity-100' : 'opacity-0 pointer-events-none'"
      >
        <button 
          @click.stop="$emit('edit', tunnel)"
          class="rounded p-1 text-slate-400 hover:bg-slate-700 hover:text-white"
          title="Edit"
        >
          <Pencil :size="16" />
        </button>
        <button 
          @click.stop="$emit('delete', tunnel.id)"
          class="rounded p-1 text-slate-400 hover:bg-red-900/50 hover:text-red-400"
          title="Delete"
        >
          <Trash2 :size="16" />
        </button>
      </div>
    </div>
  </div>
</template>
