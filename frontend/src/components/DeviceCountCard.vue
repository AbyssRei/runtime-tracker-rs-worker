<script setup>
import { computed } from 'vue';

const props = defineProps({
  devices: { type: Array, required: true }
});

const totalDevices = computed(() => props.devices.filter(d => d.device !== 'summary').length);
const onlineDevices = computed(() => props.devices.filter(d => d.running).length);
</script>

<template>
  <div class="bg-white rounded-lg not-dark:shadow-md p-6 dark:bg-[#181a1b]">
    <div class="grid grid-cols-2 gap-4">
      <div class="border border-gray-200 dark:border-[#384456] rounded-lg p-4 text-center not-dark:shadow-md">
        <h3 class="text-gray-500 text-sm font-medium flex items-center justify-center gap-1">总设备数</h3>
        <p class="text-2xl font-bold mt-2">{{ totalDevices }}</p>
      </div>
      <div class="border border-gray-200 dark:border-[#384456] rounded-lg p-4 text-center not-dark:shadow-md">
        <h3 class="text-gray-500 text-sm font-medium flex items-center justify-center gap-1">在线设备数</h3>
        <p class="text-2xl font-bold mt-2 text-green-600">{{ onlineDevices }}</p>
      </div>
    </div>
  </div>
</template>
