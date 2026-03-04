<script setup>
import { ref, watch, onMounted, onBeforeUnmount, nextTick, computed } from 'vue';
import { Chart, CategoryScale, LinearScale, BarElement, Title, Tooltip, Legend, BarController } from 'chart.js';

Chart.register(CategoryScale, LinearScale, BarElement, Title, Tooltip, Legend, BarController);

const props = defineProps({
  timeStats: { type: Array, required: true },
  timeLabels: { type: Array, required: true },
  timeDimension: { type: String, default: 'hour' },
  type: { type: String, default: 'usage' }
});

const chartRef = ref(null);
const chartInstance = { current: null };

const useHours = computed(() => {
  if (!props.timeStats || props.timeStats.length === 0) return false;
  return Math.max(...props.timeStats) >= 70;
});

const chartData = computed(() => {
  if (!props.timeStats) return [];
  return useHours.value ? props.timeStats.map(v => +(v / 60).toFixed(2)) : props.timeStats;
});

const yLabel = computed(() => {
  if (props.type === 'eyetime') return useHours.value ? '时间 (小时)' : '时间 (分钟)';
  return useHours.value ? '使用时间 (小时)' : '使用时间 (分钟)';
});

const createChart = () => {
  if (!chartRef.value || !props.timeStats || props.timeStats.length === 0) return;
  if (chartInstance.current) { chartInstance.current.destroy(); chartInstance.current = null; }
  chartInstance.current = new Chart(chartRef.value, {
    type: 'bar',
    data: {
      labels: props.timeLabels,
      datasets: [{
        label: yLabel.value,
        data: chartData.value,
        backgroundColor: 'rgba(59, 130, 246, 0.5)',
        borderColor: 'rgba(59, 130, 246, 1)',
        borderWidth: 1
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: true,
      scales: {
        y: { beginAtZero: true, title: { display: true, text: yLabel.value } },
        x: { title: { display: true, text: props.timeDimension === 'hour' ? '小时' : props.timeDimension === 'day' ? '日期' : '月份' } }
      },
      plugins: {
        legend: { display: false },
        tooltip: {
          callbacks: {
            label(context) {
              const val = context.parsed.y;
              if (useHours.value) return ` ${val} 小时`;
              return ` ${val} 分钟`;
            }
          }
        }
      }
    }
  });
};

let prevLabelCount = 0;
watch(() => props.timeLabels.length, async (newLen) => {
  if (newLen !== prevLabelCount) { prevLabelCount = newLen; await nextTick(); createChart(); }
});
watch(useHours, async () => { await nextTick(); createChart(); });
onMounted(() => { prevLabelCount = props.timeLabels.length; nextTick(createChart); });
onBeforeUnmount(() => { if (chartInstance.current) { chartInstance.current.destroy(); chartInstance.current = null; } });
</script>

<template>
  <div class="w-full">
    <canvas ref="chartRef"></canvas>
  </div>
</template>
