<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import type { TunnelConfig, DockerContainer } from '../api';
import { fetchContainers, getContainerDetails } from '../api';
import { Loader2, X } from 'lucide-vue-next';

const props = defineProps<{
  isOpen: boolean;
  editData?: TunnelConfig | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', data: TunnelConfig): void;
}>();

const mode = ref<'standard' | 'docker'>('standard');
const isLoadingContainers = ref(false);
const isFetchingDetails = ref(false);
const containers = ref<DockerContainer[]>([]);
const selectedContainerId = ref('');
const exposedPorts = ref<number[]>([]);
const errorMsg = ref('');

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
  container_id: '',
  container_name: ''
});

// Initialize form when opening
watch(() => props.isOpen, (newVal) => {
  if (newVal) {
    if (props.editData) {
      Object.assign(formData, props.editData);
      mode.value = props.editData.mode;
      if (props.editData.mode === 'docker') {
        selectedContainerId.value = props.editData.container_id || '';
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
        ssh_key_path: '', // TODO: Default from settings
        ssh_password: '',
        local_port: 8080,
        target_host: '127.0.0.1',
        target_port: 80,
        container_id: '',
        container_name: ''
      });
      mode.value = 'standard';
      containers.value = [];
      exposedPorts.value = [];
      selectedContainerId.value = '';
    }
  }
});

const handleFetchContainers = async () => {
  if (!formData.ssh_host || !formData.ssh_username) {
    errorMsg.value = 'Please fill SSH Host and Username';
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
      password: formData.ssh_password
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    isLoadingContainers.value = false;
  }
};

const selectContainer = async (container: DockerContainer) => {
  selectedContainerId.value = container.id;
  formData.container_id = container.id;
  formData.container_name = container.name;
  
  isFetchingDetails.value = true;
  exposedPorts.value = [];
  errorMsg.value = '';
  
  try {
    // 1. Get internal IP
    const details = await getContainerDetails({
      host: formData.ssh_host,
      port: formData.ssh_port,
      username: formData.ssh_username,
      auth_type: formData.auth_type,
      private_key_path: formData.ssh_key_path,
      password: formData.ssh_password
    }, container.id);
    
    formData.target_host = details.ip;
    
    // 2. Parse ports
    // Format examples: "0.0.0.0:80->80/tcp", "80/tcp", "9000/tcp, 0.0.0.0:80->80/tcp"
    const regex = /(\d+)\/tcp/g;
    const matches = [...container.ports.matchAll(regex)];
    const ports = new Set<number>();
    
    matches.forEach(m => {
        if (m[1]) ports.add(parseInt(m[1]));
    });
    
    exposedPorts.value = Array.from(ports).sort((a, b) => a - b);
    
    if (exposedPorts.value.length > 0) {
        const firstPort = exposedPorts.value[0];
        if (firstPort !== undefined) {
            formData.target_port = firstPort;
        }
    } else {
        formData.target_port = 80;
    }
    
  } catch (e) {
    errorMsg.value = "Failed to get container details: " + e;
  } finally {
    isFetchingDetails.value = false;
  }
};

const setMode = (m: string) => {
  mode.value = m as 'standard' | 'docker';
};

const save = () => {
  formData.mode = mode.value;
  emit('save', { ...formData });
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="w-full max-w-2xl rounded-xl bg-slate-900 shadow-2xl ring-1 ring-slate-700">
      
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-slate-800 p-4">
        <h2 class="text-lg font-semibold text-white">
          {{ editData ? 'Edit Tunnel' : 'Add New Tunnel' }}
        </h2>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white">
          <X :size="20" />
        </button>
      </div>

      <!-- Body -->
      <div class="max-h-[70vh] overflow-y-auto p-6">
        
        <!-- Segmented Control -->
        <div class="mb-6 flex rounded-lg bg-slate-800 p-1">
          <button 
            v-for="m in ['standard', 'docker']" 
            :key="m"
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
            <label class="mb-1 block text-xs font-medium text-slate-400">Tunnel Name</label>
            <input 
              v-model="formData.name"
              type="text" 
              class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
              placeholder="e.g. Prod Database"
              autocapitalize="off"
              autocomplete="off"
            >
          </div>

          <!-- Common: SSH Config -->
          <div class="grid grid-cols-12 gap-4">
            <div class="col-span-8">
              <label class="mb-1 block text-xs font-medium text-slate-400">SSH Host</label>
              <input 
                v-model="formData.ssh_host"
                type="text" 
                autocapitalize="off"
                autocomplete="off" 
                class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                placeholder="192.168.1.100"
              >
            </div>
            <div class="col-span-4">
              <label class="mb-1 block text-xs font-medium text-slate-400">Port</label>
              <input 
                v-model.number="formData.ssh_port"
                type="number" 
                autocapitalize="off"
                autocomplete="off" 
                class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
              >
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="mb-1 block text-xs font-medium text-slate-400">Username</label>
              <input 
                v-model="formData.ssh_username"
                type="text" 
                class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                placeholder="root"
                autocapitalize="off"
                autocomplete="off"
              >
            </div>
            <div>
              <label class="mb-1 block text-xs font-medium text-slate-400">Auth Type</label>
              <select 
                v-model="formData.auth_type"
                class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
              >
                <option value="key">Identity File (Key)</option>
                <option value="password">Password</option>
              </select>
            </div>
          </div>

          <div v-if="formData.auth_type === 'key'">
            <label class="mb-1 block text-xs font-medium text-slate-400">Private Key Path</label>
            <input 
              v-model="formData.ssh_key_path"
              type="text" 
              class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
              placeholder="/Users/me/.ssh/id_rsa"
              autocapitalize="off"
              autocomplete="off"
            >
          </div>
          <div v-else>
            <label class="mb-1 block text-xs font-medium text-slate-400">Password</label>
            <input 
              v-model="formData.ssh_password"
              type="password" 
              class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
              autocapitalize="off"
              autocomplete="off"
            >
          </div>

          <!-- Mode Specific -->
          <div v-if="mode === 'standard'" class="mt-4 border-t border-slate-800 pt-4">
            <h3 class="mb-3 text-sm font-semibold text-slate-300">Forwarding Rules</h3>
            <div class="grid grid-cols-3 gap-4">
              <div>
                <label class="mb-1 block text-xs font-medium text-slate-400">Local Port</label>
                <input 
                  v-model.number="formData.local_port"
                  type="number" 
                  class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                  autocapitalize="off"
                  autocomplete="off"
                >
              </div>
              <div>
                <label class="mb-1 block text-xs font-medium text-slate-400">Target Host</label>
                <input 
                  v-model="formData.target_host"
                  type="text" 
                  class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                  autocapitalize="off"
                  autocomplete="off"
                >
              </div>
              <div>
                <label class="mb-1 block text-xs font-medium text-slate-400">Target Port</label>
                <input 
                  v-model.number="formData.target_port"
                  type="number" 
                  class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                  autocapitalize="off"
                  autocomplete="off"
                >
              </div>
            </div>
          </div>

          <div v-else class="mt-4 border-t border-slate-800 pt-4">
            <h3 class="mb-3 text-sm font-semibold text-slate-300">Container Selection</h3>
            
            <button 
              @click="handleFetchContainers"
              :disabled="isLoadingContainers"
              class="mb-4 flex items-center justify-center gap-2 w-full rounded-md bg-blue-600 py-2 text-sm font-medium text-white hover:bg-blue-500 disabled:opacity-50"
            >
              <Loader2 v-if="isLoadingContainers" class="animate-spin" :size="16" />
              {{ isLoadingContainers ? 'Connecting...' : 'Fetch Containers' }}
            </button>

            <div v-if="errorMsg" class="mb-4 rounded bg-red-900/20 p-2 text-xs text-red-400">
              {{ errorMsg }}
            </div>

            <div v-if="containers.length > 0" class="mb-4 max-h-40 overflow-y-auto rounded-md border border-slate-700">
              <table class="w-full text-left text-xs">
                <thead class="bg-slate-800 text-slate-400">
                  <tr>
                    <th class="p-2">Name</th>
                    <th class="p-2">Image</th>
                    <th class="p-2">Ports</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800">
                  <tr 
                    v-for="c in containers" 
                    :key="c.id"
                    class="cursor-pointer hover:bg-slate-800"
                    :class="selectedContainerId === c.id ? 'bg-blue-900/30' : ''"
                    @click="selectContainer(c)"
                  >
                    <td class="p-2 text-white">{{ c.name }}</td>
                    <td class="p-2 text-slate-400 truncate max-w-[100px]">{{ c.image }}</td>
                    <td class="p-2 text-slate-400 truncate max-w-[100px]">{{ c.ports }}</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div v-if="selectedContainerId" class="mt-4 border-t border-slate-800 pt-4">
              <div v-if="isFetchingDetails" class="flex items-center justify-center py-4 text-slate-400 text-xs">
                 <Loader2 class="animate-spin mr-2" :size="16" /> Resolving container details...
              </div>
              <div v-else class="grid grid-cols-2 gap-4">
                 <div>
                    <label class="mb-1 block text-xs font-medium text-slate-400">Exposed Port</label>
                    <select 
                      v-if="exposedPorts.length > 0"
                      v-model.number="formData.target_port"
                      class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                    >
                      <option v-for="p in exposedPorts" :key="p" :value="p">{{ p }}</option>
                    </select>
                    <input 
                      v-else
                      v-model.number="formData.target_port"
                      type="number"
                      class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                      placeholder="e.g. 80"
                      autocapitalize="off"
                      autocomplete="off"
                    >
                 </div>
                 <div>
                    <label class="mb-1 block text-xs font-medium text-slate-400">Local Port</label>
                    <input 
                      v-model.number="formData.local_port"
                      type="number" 
                      class="w-full rounded-md border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none"
                      autocapitalize="off"
                      autocomplete="off"
                    >
                 </div>
              </div>
            </div>
          </div>

        </div>
      </div>

      <!-- Footer -->
      <div class="flex justify-end gap-3 border-t border-slate-800 p-4">
        <button 
          @click="$emit('close')"
          class="rounded-md px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-800"
        >
          Cancel
        </button>
        <button 
          @click="save"
          class="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-500"
        >
          Save Tunnel
        </button>
      </div>

    </div>
  </div>
</template>
