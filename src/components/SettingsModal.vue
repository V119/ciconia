<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { X, Save } from 'lucide-vue-next';
import { enable, disable } from '@tauri-apps/plugin-autostart';
import { type AppSettings, getSettings, saveSettings } from '../api';
import { applyTheme } from '../utils/theme';
import Switch from './ui/Switch.vue';

const props = defineProps<{ isOpen: boolean }>();
const emit = defineEmits(['close']);

const activeTab = ref<'general' | 'network' | 'appearance'>('general');
const settings = ref<AppSettings | null>(null);
const loading = ref(false);

const loadSettings = async () => {
  loading.value = true;
  try {
    settings.value = await getSettings();
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
};

const handleSave = async () => {
  if (!settings.value) return;
  try {
    // Handle Autostart
    try {
      if (settings.value.launch_at_login) {
        await enable();
      } else {
        await disable();
      }
    } catch (e) {
      console.warn('Failed to toggle autostart:', e);
    }

    await saveSettings(settings.value);
    applyTheme(settings.value.theme as 'system' | 'dark' | 'light');
    emit('close');
  } catch (e) {
    alert('Failed to save settings: ' + e);
  }
};

watch(() => settings.value?.theme, (newTheme) => {
  if (newTheme) {
    applyTheme(newTheme as 'system' | 'dark' | 'light');
  }
});

watch(() => props.isOpen, (newVal) => {
  if (newVal) {
    loadSettings();
  }
});

onMounted(() => {
  if (props.isOpen) {
    loadSettings();
  }
});
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="flex h-[600px] w-full max-w-2xl flex-col rounded-xl bg-slate-900 shadow-2xl ring-1 ring-slate-700">
      
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-slate-800 p-4">
        <h2 class="text-lg font-semibold text-white">Global Settings</h2>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white">
          <X :size="20" />
        </button>
      </div>

      <div class="flex flex-1 overflow-hidden">
        <!-- Sidebar -->
        <div class="w-48 border-r border-slate-800 bg-slate-900/50 p-4">
          <nav class="space-y-1">
            <button 
              @click="activeTab = 'general'"
              class="w-full rounded-md px-3 py-2 text-left text-sm font-medium transition-colors"
              :class="activeTab === 'general' ? 'bg-slate-800 text-white' : 'text-slate-400 hover:bg-slate-800/50 hover:text-slate-200'"
            >
              General
            </button>
            <button 
              @click="activeTab = 'network'"
              class="w-full rounded-md px-3 py-2 text-left text-sm font-medium transition-colors"
              :class="activeTab === 'network' ? 'bg-slate-800 text-white' : 'text-slate-400 hover:bg-slate-800/50 hover:text-slate-200'"
            >
              Network
            </button>
            <button 
              @click="activeTab = 'appearance'"
              class="w-full rounded-md px-3 py-2 text-left text-sm font-medium transition-colors"
              :class="activeTab === 'appearance' ? 'bg-slate-800 text-white' : 'text-slate-400 hover:bg-slate-800/50 hover:text-slate-200'"
            >
              Appearance
            </button>
          </nav>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6">
          <div v-if="!settings" class="flex h-full items-center justify-center text-slate-500">
            Loading...
          </div>
          
          <div v-else class="space-y-6">
            
            <!-- General Tab -->
            <div v-if="activeTab === 'general'" class="space-y-6">
              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-slate-200">Launch at Login</label>
                  <p class="text-xs text-slate-500">Automatically start application when you log in</p>
                </div>
                <Switch v-model="settings.launch_at_login" />
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-slate-200">Minimize to Tray</label>
                  <p class="text-xs text-slate-500">Hide window to tray when clicking close button</p>
                </div>
                <Switch v-model="settings.minimize_to_tray_on_close" />
              </div>

              <div>
                <label class="block text-sm font-medium text-slate-200">Keep-alive Interval</label>
                <div class="mt-2 flex items-center gap-4">
                  <input 
                    type="range" 
                    v-model.number="settings.keep_alive_interval" 
                    min="10" 
                    max="300" 
                    step="10"
                    class="h-2 w-full cursor-pointer appearance-none rounded-lg bg-slate-700 accent-blue-500"
                  >
                  <span class="w-12 text-right text-sm text-slate-400">{{ settings.keep_alive_interval }}s</span>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-slate-200">Default SSH Key</label>
                <input 
                  type="text" 
                  v-model="settings.default_ssh_key" 
                  placeholder="~/.ssh/id_rsa"
                  class="mt-1 block w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-200 placeholder-slate-500 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
              </div>
            </div>

            <!-- Network Tab -->
            <div v-if="activeTab === 'network'" class="space-y-6">
              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-slate-200">Strict Host Key Checking</label>
                  <p class="text-xs text-slate-500">Refuse to connect to unknown hosts</p>
                </div>
                <Switch v-model="settings.strict_host_key_checking" />
              </div>

              <div>
                <label class="block text-sm font-medium text-slate-200">Connection Timeout</label>
                <div class="mt-2 flex items-center gap-4">
                  <input 
                    type="range" 
                    v-model.number="settings.connection_timeout" 
                    min="5" 
                    max="60" 
                    step="5"
                    class="h-2 w-full cursor-pointer appearance-none rounded-lg bg-slate-700 accent-blue-500"
                  >
                  <span class="w-12 text-right text-sm text-slate-400">{{ settings.connection_timeout }}s</span>
                </div>
              </div>

              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-slate-200">Auto Reconnect</label>
                  <p class="text-xs text-slate-500">Automatically try to reconnect on disconnect</p>
                </div>
                <Switch v-model="settings.auto_reconnect" />
              </div>
            </div>

            <!-- Appearance Tab -->
            <div v-if="activeTab === 'appearance'" class="space-y-6">
              <div>
                <label class="block text-sm font-medium text-slate-200">Theme</label>
                <select 
                  v-model="settings.theme"
                  class="mt-1 block w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-200 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
                  <option value="system">System Default</option>
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-slate-200">Language</label>
                <select 
                  v-model="settings.language"
                  class="mt-1 block w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-200 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
                  <option value="en">English</option>
                  <option value="zh">Chinese (简体中文)</option>
                </select>
              </div>
            </div>

          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="border-t border-slate-800 p-4">
        <div class="flex justify-end gap-3">
          <button 
            @click="$emit('close')"
            class="rounded-md px-4 py-2 text-sm font-medium text-slate-300 hover:bg-slate-800 hover:text-white"
          >
            Cancel
          </button>
          <button 
            @click="handleSave"
            class="flex items-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500"
          >
            <Save :size="16" />
            Save Changes
          </button>
        </div>
      </div>

    </div>
  </div>
</template>
