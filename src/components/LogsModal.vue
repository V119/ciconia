<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { X, Pause, Play, Trash2, Copy } from 'lucide-vue-next';

defineProps<{ isOpen: boolean }>();
defineEmits(['close']);

interface LogEntry {
  id: string;
  line: string;
  level: string;
  timestamp: string;
}

const logs = ref<LogEntry[]>([]);
const isPaused = ref(false);
const logsContainer = ref<HTMLElement | null>(null);

let unlisten: (() => void) | null = null;

const addLog = (entry: LogEntry) => {
  if (isPaused.value) return;
  logs.value.push(entry);
  if (logs.value.length > 1000) {
    logs.value = logs.value.slice(-1000); // Keep last 1000 logs
  }
  scrollToBottom();
};

const scrollToBottom = async () => {
  await nextTick();
  if (logsContainer.value) {
    logsContainer.value.scrollTop = logsContainer.value.scrollHeight;
  }
};

onMounted(async () => {
  unlisten = await listen('tunnel-log', (event: any) => {
    const payload = event.payload;
    addLog({
      id: payload.id,
      line: payload.line,
      level: payload.level,
      timestamp: new Date().toLocaleTimeString(),
    });
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

const clearLogs = () => {
  logs.value = [];
};

const copyLogs = () => {
  const text = logs.value.map(l => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.line}`).join('\n');
  navigator.clipboard.writeText(text);
};

const togglePause = () => {
  isPaused.value = !isPaused.value;
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="flex h-[80vh] w-full max-w-4xl flex-col rounded-xl bg-black shadow-2xl ring-1 ring-slate-700">
      
      <!-- Toolbar -->
      <div class="flex items-center justify-between border-b border-slate-800 bg-slate-900 p-2">
        <div class="flex gap-2">
           <button 
             @click="togglePause"
             class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white" 
             :title="isPaused ? 'Resume' : 'Pause'"
           >
             <component :is="isPaused ? Play : Pause" :size="16" />
           </button>
           <button @click="clearLogs" class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white" title="Clear">
             <Trash2 :size="16" />
           </button>
           <button @click="copyLogs" class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white" title="Copy">
             <Copy :size="16" />
           </button>
        </div>
        <div class="text-xs font-medium text-slate-500">
          Global Logs
        </div>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white">
          <X :size="20" />
        </button>
      </div>

      <!-- Logs -->
      <div ref="logsContainer" class="flex-1 overflow-y-auto p-4 font-mono text-xs text-green-500">
        <div v-for="(log, index) in logs" :key="index" class="whitespace-pre-wrap break-all">
          <span class="text-slate-500">[{{ log.timestamp }}]</span>
          <span :class="log.level === 'error' ? 'text-red-400' : 'text-green-400'" class="ml-2">
            {{ log.line }}
          </span>
        </div>
        <div v-if="logs.length === 0" class="text-slate-600 italic">
          Waiting for logs...
        </div>
      </div>
    </div>
  </div>
</template>
