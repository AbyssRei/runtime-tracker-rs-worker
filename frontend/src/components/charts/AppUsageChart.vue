<script setup>
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { Chart, ArcElement, Tooltip, Legend, DoughnutController } from 'chart.js';

Chart.register(ArcElement, Tooltip, Legend, DoughnutController);

const props = defineProps({
  appStats: { type: Object, required: true },
  totalUsage: { type: Number, required: true }
});

const chartRef = ref(null);
const chartInstance = { current: null };

const generateColors = (count) => {
  const colors = [
    '#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6',
    '#EC4899', '#14B8A6', '#F97316', '#6366F1', '#84CC16'
  ];
  while (colors.length < count) colors.push(`hsl(${Math.random() * 360}, 70%, 60%)`);
  return colors.slice(0, count);
};

const createChart = () => {
  if (!chartRef.value || !props.appStats) return;
  if (chartInstance.current) {
    chartInstance.current.destroy();
    chartInstance.current = null;
  }
  const sortedApps = Object.entries(props.appStats)
    .sort(([, a], [, b]) => b - a);
  const topApps = sortedApps.slice(0, 8);
  const otherApps = sortedApps.slice(8);
  const otherTotal = otherApps.reduce((sum, [, val]) => sum + val, 0);
  const labels = topApps.map(([name]) => name);
  const data = topApps.map(([, val]) => val);
  if (otherTotal > 0) { labels.push('其他'); data.push(otherTotal); }
  const colors = generateColors(labels.length);
  chartInstance.current = new Chart(chartRef.value, {
    type: 'doughnut',
    data: { labels, datasets: [{ data, backgroundColor: colors, borderWidth: 1 }] },
    options: {
      responsive: true,
      maintainAspectRatio: true,
      plugins: {
        legend: { position: 'bottom', labels: { boxWidth: 12, padding: 10, font: { size: 11 } } },
        tooltip: {
          callbacks: {
            label(context) {
              const value = context.parsed;
              const total = context.dataset.data.reduce((a, b) => a + b, 0);
              const pct = ((value / total) * 100).toFixed(1);
              const h = Math.floor(value / 60); const m = value % 60;
              const time = h > 0 ? `${h}时${m}分` : `${m}分`;
              return ` ${context.label}: ${time} (${pct}%)`;
            }
          }
        }
      }
    }
  });
};

watch(() => props.appStats, async () => { await nextTick(); createChart(); }, { deep: true });
onMounted(() => { nextTick(createChart); });
onBeforeUnmount(() => { if (chartInstance.current) { chartInstance.current.destroy(); chartInstance.current = null; } });
</script>

<template>
  <div class="w-full flex justify-center items-center">
    <canvas ref="chartRef" style="max-height: 300px;"></canvas>
  </div>
</template>
