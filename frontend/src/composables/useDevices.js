import { ref, computed, onUnmounted } from 'vue';
import config from '../config.js';

const API_BASE = config.API_BASE;

const OVERVIEW_DEVICE = {
    device: 'summary',
    currentApp: '不告诉你'
};

export function useDevices(onError, { showSummary }) {
    const devices = ref([]);
    const selectedDevice = ref(null);
    const clientIp = ref('获取中...');
    const refreshInterval = ref(null);
    const isRefreshing = ref(false);
    const statsKey = ref(0);

    const hasRealDevices = computed(() => {
        return devices.value.some(d => d.device !== 'summary');
    });

    const fetchClientIp = async () => {
        try {
            const response = await fetch(`${API_BASE}/ip`);
            const data = await response.json();
            clientIp.value = data.ip || '未知';
        } catch (err) {
            clientIp.value = '获取失败';
            console.error('获取IP地址失败:', err);
        }
    };

    const fetchDevices = async () => {
        try {
            const response = await fetch(`${API_BASE}/devices`);
            if (!response.ok) {
                throw new Error(`请求失败: ${response.status}`);
            }
            const realDevices = await response.json();
            if (showSummary.value) {
                devices.value = [OVERVIEW_DEVICE, ...realDevices];
            } else {
                devices.value = realDevices;
            }
            if (devices.value.length > 0 && !selectedDevice.value) {
                selectedDevice.value = devices.value[0].device;
            }
        } catch (err) {
            onError?.(`获取设备列表失败: ${err.message}`);
            console.error('获取设备列表错误:', err);
            devices.value = [];
        }
    };

    const refreshStats = () => {
        if (selectedDevice.value) {
            isRefreshing.value = true;
            statsKey.value++;
            setTimeout(() => {
                isRefreshing.value = false;
            }, 500);
        }
    };

    const setupAutoRefresh = () => {
        if (refreshInterval.value) {
            clearInterval(refreshInterval.value);
        }
        refreshInterval.value = setInterval(() => {
            if (selectedDevice.value) {
                fetchDevices();
            }
        }, 30000);
    };

    const getSelectedDevice = () => {
        if (!selectedDevice.value) return null;
        return devices.value.find(device => device.device === selectedDevice.value) || null;
    };

    onUnmounted(() => {
        if (refreshInterval.value) {
            clearInterval(refreshInterval.value);
        }
    });

    return {
        devices,
        selectedDevice,
        clientIp,
        isRefreshing,
        statsKey,
        hasRealDevices,
        fetchClientIp,
        fetchDevices,
        refreshStats,
        setupAutoRefresh,
        getSelectedDevice
    };
}
