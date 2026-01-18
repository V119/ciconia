<script setup lang="ts">
import {ref, reactive, watch, onMounted} from 'vue';
import {type TunnelConfig, type DockerContainer, type AppSettings, getSettings} from '../api';
import { fetchContainers } from '../api';
import { Loader2, X } from 'lucide-vue-next';

const props = defineProps<{
  isOpen: boolean;
  editData?: TunnelConfig | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', data: TunnelConfig): void;
}>();

// 提取公共样式，保持模板整洁
const INPUT_CLASSES = "w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none transition-colors placeholder-slate-500";
const LABEL_CLASSES = "mb-1 block text-xs font-medium text-slate-400";

const mode = ref<'standard' | 'docker'>('standard');
const isLoadingContainers = ref(false);
const containers = ref<DockerContainer[]>([]);
const selectedContainerId = ref('');
const selectedPort = ref<string>('');
const exposedPorts = ref<number[]>([]);
const errorMsg = ref('');
const searchKeyword = ref('');
const settings = ref<AppSettings | null>(null);

const loadSettings = async () => {
  try {
    settings.value = await getSettings();
  } catch (e) {
    console.error(e);
  }
};

const formData = reactive<TunnelConfig>({
  id: '',
  name: '',
  mode: 'standard',
  ssh_host: '',
  ssh_port: 22,
  ssh_username: 'root',
  auth_type: 'key',
  ssh_key_path: '',
  ssh_password: '',
  local_port: 8080,
  target_host: '127.0.0.1',
  target_port: 80,
  container_name: null,
  container_port: null
});

// Initialize form when opening
watch(() => props.isOpen, async (newVal) => {
  if (newVal) {
    if (!settings.value) {
      await loadSettings();
    }

    errorMsg.value = '';
    if (props.editData) {
      Object.assign(formData, props.editData);
      mode.value = props.editData.mode;
      if (props.editData.mode === 'docker') {
        selectedContainerId.value = props.editData.container_name || '';
        formData.container_port = props.editData.container_port || null;
      }
    } else {
      // Reset
      Object.assign(formData, {
        id: crypto.randomUUID(),
        name: '',
        mode: 'standard',
        ssh_host: '',
        ssh_port: 22,
        ssh_username: 'root',
        auth_type: 'key',
        ssh_key_path: settings.value?.default_ssh_key || '',
        ssh_password: '',
        local_port: 8080,
        target_host: '127.0.0.1',
        target_port: 80,
        container_name: '',
        container_port: null
      });
      mode.value = 'standard';
      containers.value = [];
      exposedPorts.value = [];
      selectedContainerId.value = '';
      searchKeyword.value = '';
    }
  }
});

onMounted(() => {
  if (props.isOpen) {
    loadSettings();
  }
});

const handleFetchContainers = async () => {
  if (!formData.ssh_host || !formData.ssh_username) {
    errorMsg.value = 'Please fill SSH Host and Username first';
    return;
  }

  isLoadingContainers.value = true;
  errorMsg.value = '';
  containers.value = [];
  selectedContainerId.value = '';

  try {
    containers.value = await fetchContainers({
      host: formData.ssh_host,
      port: formData.ssh_port,
      username: formData.ssh_username,
      auth_type: formData.auth_type,
      private_key_path: formData.ssh_key_path,
      password: formData.ssh_password,
      keyword: searchKeyword.value || undefined
    });
  } catch (e) {
    errorMsg.value = e instanceof Error ? e.message : String(e);
  } finally {
    isLoadingContainers.value = false;
  }
};

const selectPort = (container: DockerContainer, port: string) => {
  selectedContainerId.value = container.id;
  selectedPort.value = port;
  formData.container_name = container.name;

  // Extract port logic
  const match = port.match(/:(\d+)\/tcp/);
  let portNumber: number;

  if (match && match[1]) {
    portNumber = parseInt(match[1]);
  } else {
    const match2 = port.match(/^(\d+)\/tcp$/);
    portNumber = (match2 && match2[1]) ? parseInt(match2[1]) : 80;
  }

  formData.target_port = portNumber;
  formData.container_port = portNumber;
};

watch(() => formData.target_port, (newVal) => {
  if (mode.value === 'docker') {
    formData.container_port = newVal;
  }
});

const setMode = (m: string) => {
  mode.value = m as 'standard' | 'docker';
};

const save = () => {
  if (mode.value === 'docker' && exposedPorts.value.length > 0) {
    exposedPorts.value.forEach(port => {
      let tunnelData = { ...formData };
      tunnelData.target_host = '';
      tunnelData.target_port = null;
      tunnelData.container_port = port;
      tunnelData.local_port = port;
      tunnelData.name = `${formData.name} - Port ${port}`;
      emit('save', tunnelData);
    });
  } else {
    let tunnelData = { ...formData };
    if (mode.value === 'docker') {
      tunnelData.target_host = '';
      tunnelData.target_port = null;
    }
    tunnelData.mode = mode.value;
    emit('save', tunnelData);
  }
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
    <div class="w-full max-w-2xl flex flex-col max-h-[90vh] rounded-xl bg-slate-900 shadow-2xl ring-1 ring-slate-700">

      <!-- Header -->
      <div class="flex items-center justify-between border-b border-slate-800 p-4 shrink-0">
        <h2 class="text-lg font-semibold text-white">
          {{ editData ? 'Edit Tunnel' : 'Add New Tunnel' }}
        </h2>
        <button
            type="button"
            @click="$emit('close')"
            class="text-slate-400 hover:text-white transition-colors"
        >
          <X :size="20" />
        </button>
      </div>

      <!-- Body -->
      <div class="flex-1 overflow-y-auto p-6">

        <!-- Mode Switcher -->
        <div class="mb-6 flex rounded-lg bg-slate-800 p-1">
          <button
              v-for="m in ['standard', 'docker']"
              :key="m"
              type="button"
              class="flex-1 rounded-md py-1.5 text-sm font-medium transition-all"
              :class="mode === m ? 'bg-slate-600 text-white shadow' : 'text-slate-400 hover:text-slate-200'"
              @click="setMode(m)"
          >
            {{ m === 'standard' ? 'Standard Mode' : 'Container Mode' }}
          </button>
        </div>

        <div class="grid gap-4">
          <!-- Common: Name -->
          <div>
            <label :class="LABEL_CLASSES">Tunnel Name</label>
            <input
                v-model="formData.name"
                type="text"
                :class="INPUT_CLASSES"
                placeholder="e.g. Prod Database"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                autocomplete="off"
            >
          </div>

          <!-- Common: SSH Config -->
          <div class="grid grid-cols-12 gap-4">
            <div class="col-span-8">
              <label :class="LABEL_CLASSES">SSH Host</label>
              <input
                  v-model="formData.ssh_host"
                  type="text"
                  :class="INPUT_CLASSES"
                  placeholder="192.168.1.100"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                  autocomplete="off"
              >
            </div>
            <div class="col-span-4">
              <label :class="LABEL_CLASSES">Port</label>
              <input
                  v-model.number="formData.ssh_port"
                  type="number"
                  :class="INPUT_CLASSES"
                  autocapitalize="off"
                  autocorrect="off"
                  autocomplete="off"
              >
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label :class="LABEL_CLASSES">Username</label>
              <input
                  v-model="formData.ssh_username"
                  type="text"
                  :class="INPUT_CLASSES"
                  placeholder="root"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                  autocomplete="off"
              >
            </div>
            <div>
              <label :class="LABEL_CLASSES">Auth Type</label>
              <select
                  v-model="formData.auth_type"
                  :class="INPUT_CLASSES"
              >
                <option value="key">Identity File (Key)</option>
                <option value="password">Password</option>
              </select>
            </div>
          </div>

          <div v-if="formData.auth_type === 'key'">
            <label :class="LABEL_CLASSES">Private Key Path</label>
            <input
                v-model="formData.ssh_key_path"
                type="text"
                :class="INPUT_CLASSES"
                placeholder="/Users/me/.ssh/id_rsa"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                autocomplete="off"
            >
          </div>
          <div v-else>
            <label :class="LABEL_CLASSES">Password</label>
            <input
                v-model="formData.ssh_password"
                type="password"
                :class="INPUT_CLASSES"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                autocomplete="new-password"
            >
          </div>

          <!-- Standard Mode Specific -->
          <div v-if="mode === 'standard'" class="mt-4 border-t border-slate-800 pt-4">
            <h3 class="mb-3 text-sm font-semibold text-slate-300">Forwarding Rules</h3>
            <div class="grid grid-cols-3 gap-4">
              <div>
                <label :class="LABEL_CLASSES">Local Port</label>
                <input
                    v-model.number="formData.local_port"
                    type="number"
                    :class="INPUT_CLASSES"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                >
              </div>
              <div>
                <label :class="LABEL_CLASSES">Target Host</label>
                <input
                    v-model="formData.target_host"
                    type="text"
                    :class="INPUT_CLASSES"
                    placeholder="127.0.0.1"
                    autocapitalize="off"
                    autocorrect="off"
                    spellcheck="false"
                    autocomplete="off"
                >
              </div>
              <div>
                <label :class="LABEL_CLASSES">Target Port</label>
                <input
                    v-model.number="formData.target_port"
                    type="number"
                    :class="INPUT_CLASSES"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                >
              </div>
            </div>
          </div>

          <!-- Docker Mode Specific -->
          <div v-else class="mt-4 border-t border-slate-800 pt-4">
            <h3 class="mb-3 text-sm font-semibold text-slate-300">Container Selection</h3>

            <div v-if="formData.container_name" class="mb-4 text-xs rounded bg-blue-900/30 border border-blue-800 p-2 text-blue-200">
              Selected: <span class="font-semibold text-white">{{ formData.container_name }}</span> (Port: {{ formData.container_port }})
            </div>

            <div class="mb-4">
              <label :class="LABEL_CLASSES">Search Keyword (Optional)</label>
              <div class="grid grid-cols-[1fr_auto] gap-2">
                <input
                    v-model="searchKeyword"
                    type="text"
                    :class="INPUT_CLASSES"
                    placeholder="e.g. nginx, redis"
                    autocapitalize="off"
                    autocorrect="off"
                    spellcheck="false"
                    autocomplete="off"
                >
                <button
                    type="button"
                    @click="handleFetchContainers"
                    :disabled="isLoadingContainers"
                    class="flex items-center justify-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  <Loader2 v-if="isLoadingContainers" class="animate-spin" :size="16" />
                  {{ isLoadingContainers ? 'Connecting...' : 'Fetch' }}
                </button>
              </div>
            </div>

            <div v-if="errorMsg" class="mb-4 rounded bg-red-900/20 border border-red-900/50 p-3 text-xs text-red-400">
              {{ errorMsg }}
            </div>

            <!-- Optimized Table Structure -->
            <div v-if="containers.length > 0" class="mb-4 max-h-40 overflow-y-auto rounded-md border border-slate-700 bg-slate-900">
              <table class="w-full text-left text-xs border-collapse">
                <thead class="bg-slate-800 text-slate-400 sticky top-0 z-10">
                <tr>
                  <th class="p-2 border-b border-slate-700">Name</th>
                  <th class="p-2 border-b border-slate-700">Image</th>
                  <th class="p-2 border-b border-slate-700">Port</th>
                </tr>
                </thead>
                <tbody class="divide-y divide-slate-800">
                <!-- 使用 template 避免非法 HTML 嵌套 -->
                <template v-for="container in containers" :key="container.id">
                  <tr
                      v-for="(port, index) in container.ports"
                      :key="`${container.id}-${port}`"
                      :class="{
                        'bg-blue-900/30': selectedContainerId === container.id && selectedPort === port,
                        'hover:bg-slate-800': !(selectedContainerId === container.id && selectedPort === port)
                      }"
                      @click="selectPort(container, port)"
                      class="cursor-pointer transition-colors"
                  >
                    <!-- Rowspan 只在第一行渲染 -->
                    <td
                        v-if="index === 0"
                        :rowspan="container.ports.length"
                        class="p-2 text-white border-r border-slate-800/50 align-top"
                    >
                      {{ container.name }}
                    </td>
                    <td
                        v-if="index === 0"
                        :rowspan="container.ports.length"
                        class="p-2 text-slate-400 truncate max-w-[120px] border-r border-slate-800/50 align-top"
                        :title="container.image"
                    >
                      {{ container.image }}
                    </td>
                    <td class="p-2 text-slate-300">
                      {{ port }}
                    </td>
                  </tr>
                </template>
                </tbody>
              </table>
            </div>

            <div v-if="selectedContainerId" class="mt-4 border-t border-slate-800 pt-4">
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label :class="LABEL_CLASSES">Exposed Port</label>
                  <select
                      v-if="exposedPorts.length > 0"
                      v-model.number="formData.target_port"
                      :class="INPUT_CLASSES"
                  >
                    <option v-for="p in exposedPorts" :key="p" :value="p">{{ p }}</option>
                  </select>
                  <input
                      v-else
                      v-model.number="formData.target_port"
                      type="number"
                      :class="INPUT_CLASSES"
                      placeholder="e.g. 80"
                      autocapitalize="off"
                      autocorrect="off"
                      autocomplete="off"
                  >
                </div>
                <div>
                  <label :class="LABEL_CLASSES">Local Port</label>
                  <input
                      v-model.number="formData.local_port"
                      type="number"
                      :class="INPUT_CLASSES"
                      autocapitalize="off"
                      autocorrect="off"
                      autocomplete="off"
                  >
                </div>
              </div>
            </div>
          </div>

        </div>
      </div>

      <!-- Footer -->
      <div class="flex justify-end gap-3 border-t border-slate-800 p-4 shrink-0">
        <button
            type="button"
            @click="$emit('close')"
            class="rounded-md px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-800 transition-colors"
        >
          Cancel
        </button>
        <button
            type="button"
            @click="save"
            class="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-500 transition-colors"
        >
          Save Tunnel
        </button>
      </div>

    </div>
  </div>
</template>