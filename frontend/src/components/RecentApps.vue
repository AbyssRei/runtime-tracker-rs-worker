<script setup>
import { ref, onMounted, watch, computed } from 'vue';
import config from '../config.js'
const API_BASE = config.API_BASE

const props = defineProps({
  deviceId: { type: String, required: true },
  refreshTrigger: { type: Number, default: 0 }
});

const recentApps = ref([]);
const error = ref(null);
const isExpanded = ref(false);
const showLimit = ref(5);

const processedApps = computed(() => {
  const sortedApps = [...recentApps.value].sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
  return sortedApps.map((app, index, arr) => {
    const startTime = new Date(app.timestamp);
    let endTime = null;
    let duration = null;
    if (app.running && index === 0) {
      duration = Math.floor((new Date() - startTime) / 1000);
    } else if (index > 0) {
      endTime = new Date(arr[index - 1].timestamp);
      duration = Math.floor((endTime - startTime) / 1000);
    }
    return { ...app, startTime: app.timestamp, endTime: endTime ? endTime.toISOString() : null, duration };
  });
});

const displayedApps = computed(() => {
  if (isExpanded.value || processedApps.value.length <= showLimit.value) return processedApps.value;
  return processedApps.value.slice(0, showLimit.value);
});

const shouldShowToggle = computed(() => processedApps.value.length > showLimit.value);
const toggleExpanded = () => { isExpanded.value = !isExpanded.value; };

const fetchRecentApps = async () => {
  try {
    error.value = null;
    const response = await fetch(`${API_BASE}/recent/${props.deviceId}`);
    if (!response.ok) throw new Error('获取最近应用失败');
    const data = await response.json();
    recentApps.value = data.data;
  } catch (err) {
    error.value = `获取最近应用失败: ${err.message}`;
  }
};

const formatDuration = (seconds) => {
  if (seconds === null) return '设备待机';
  if (seconds < 60) return `${seconds}秒`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}分钟`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}小时${minutes}分钟`;
};

const formatTime = (isoString) => {
  if (!isoString) return '未结束';
  return new Date(isoString).toLocaleString();
};

onMounted(fetchRecentApps);
watch(() => props.deviceId, fetchRecentApps);
watch(() => props.refreshTrigger, fetchRecentApps);
</script>

<template>
  <div class="mt-8 rounded-lg border-2 border-gray-200 shadow-md p-6 relative dark:border-gray-700">
    <div class="flex justify-between items-center mb-4">
      <h3 class="text-lg font-medium">最近使用的应用</h3>
    </div>
    <div v-if="error" class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative mb-4">{{ error }}</div>
    <div class="overflow-x-auto">
      <div v-if="recentApps.length === 0" class="text-center py-4 text-gray-500">暂无最近使用应用记录</div>
      <div v-if="recentApps.length > 0">
        <table class="min-w-full not-dark:divide-y divide-gray-200">
          <thead>
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">应用</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">开始时间</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">结束时间</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">持续时间</th>
            </tr>
          </thead>
          <tbody class="not-dark:divide-y divide-gray-50">
            <tr v-for="app in displayedApps" :key="app._id">
              <td class="px-6 py-4 whitespace-nowrap text-sm font-medium truncate max-w-xs">{{ app.appName }}</td>
              <td class="px-6 py-4 whitespace-nowrap text-sm">{{ formatTime(app.startTime) }}</td>
              <td class="px-6 py-4 whitespace-nowrap text-sm">{{ app.endTime ? formatTime(app.endTime) : '运行中' }}</td>
              <td class="px-6 py-4 whitespace-nowrap text-sm">{{ formatDuration(app.duration) }}</td>
            </tr>
          </tbody>
        </table>
        <div v-if="shouldShowToggle" class="flex justify-center mt-4">
          <button @click="toggleExpanded" class="flex items-center px-4 py-2 text-sm text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors duration-200 dark:text-blue-400 dark:bg-blue-900/20 dark:hover:bg-blue-900/30">
            <template v-if="isExpanded">收起 (显示前{{ showLimit }}条)</template>
            <template v-else>展开查看全部 ({{ processedApps.length }}条)</template>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
