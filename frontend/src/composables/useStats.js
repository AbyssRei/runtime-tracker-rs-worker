import { ref } from 'vue';
import config from '../config.js';

const API_BASE = config.API_BASE;

export function useStats() {
    const stats = ref(null);
    const error = ref(null);
    const loading = ref(false);

    const getTimezoneOffset = () => {
        const offset = new Date().getTimezoneOffset();
        return -offset / 60;
    };

    const isEyetimeMode = (mode) => mode !== 'single';

    const fetchDailyStats = async (deviceId, date = null, mode = 'single') => {
        loading.value = true;
        error.value = null;
        try {
            const offsetHours = getTimezoneOffset();
            const timezoneParam = `timezoneOffset=${offsetHours > 0 ? '+' : ''}${offsetHours}`;
            let url = `${API_BASE}/stats/${deviceId}?${timezoneParam}`;
            if (date) {
                const today = new Date().toISOString().split('T')[0];
                if (mode === 'single') {
                    url = date === today
                        ? `${API_BASE}/stats/${deviceId}?${timezoneParam}`
                        : `${API_BASE}/stats/${deviceId}?date=${date}&${timezoneParam}`;
                } else {
                    url = date === today
                        ? `${API_BASE}/eyetime/daily/?${timezoneParam}`
                        : `${API_BASE}/eyetime/daily?date=${date}&${timezoneParam}`;
                }
            }
            const response = await fetch(url);
            if (!response.ok) throw new Error('获取统计失败');
            const data = await response.json();
            stats.value = transformDailyData(data, isEyetimeMode(mode));
        } catch (err) {
            error.value = `获取统计信息失败: ${err.message}`;
        } finally {
            loading.value = false;
        }
    };

    const fetchPeriodStats = async (periodType, deviceId, offset, mode, transformFn) => {
        loading.value = true;
        error.value = null;
        try {
            const offsetHours = getTimezoneOffset();
            const offsetParam = periodType === 'weekly' ? 'weekOffset' : 'monthOffset';
            let url;
            if (mode === 'single') {
                url = `${API_BASE}/${periodType}/${deviceId}?${offsetParam}=${offset}&timezoneOffset=${offsetHours}`;
            } else {
                url = `${API_BASE}/eyetime/${periodType}?${offsetParam}=${offset}&timezoneOffset=${offsetHours}`;
            }
            const response = await fetch(url);
            if (!response.ok) throw new Error(`获取${periodType === 'weekly' ? '周' : '月'}统计失败`);
            const data = await response.json();
            stats.value = transformFn(data, isEyetimeMode(mode));
        } catch (err) {
            error.value = `获取${periodType === 'weekly' ? '周' : '月'}统计信息失败: ${err.message}`;
        } finally {
            loading.value = false;
        }
    };

    const fetchWeeklyStats = async (deviceId, weekOffset = 0, mode = 'single') => {
        await fetchPeriodStats('weekly', deviceId, weekOffset, mode, transformWeeklyData);
    };

    const fetchMonthlyStats = async (deviceId, monthOffset = 0, mode = 'single') => {
        await fetchPeriodStats('monthly', deviceId, monthOffset, mode, transformMonthlyData);
    };

    const transformDailyData = (data, eyetimeMode = false) => {
        return {
            type: 'daily',
            dateRange: {
                start: data.date || new Date().toISOString().split('T')[0],
                end: data.date || new Date().toISOString().split('T')[0]
            },
            totalUsage: data.totalUsage,
            appStats: eyetimeMode ? null : data.appStats,
            timeStats: data.hourlyStats,
            timeLabels: Array.from({length: 24}, (_, i) => `${i}时`),
            timeDimension: 'hour'
        };
    };

    const transformPeriodData = (data, type, dateRangeKey, timeDimension, eyetimeMode = false) => {
        let appStats = {};
        if (!eyetimeMode && data.appDailyStats) {
            Object.entries(data.appDailyStats).forEach(([appName, dailyData]) => {
                appStats[appName] = Object.values(dailyData).reduce((sum, val) => sum + val, 0);
            });
        } else {
            appStats = null;
        }
        const dates = Object.keys(data.dailyTotals || {}).sort();
        const timeLabels = dates.map(date => {
            const d = new Date(date);
            return `${d.getMonth() + 1}/${d.getDate()}`;
        });
        const timeStats = dates.map(date => data.dailyTotals[date] || 0);
        const totalUsage = appStats ? Object.values(appStats).reduce((sum, val) => sum + val, 0)
            : timeStats.reduce((sum, val) => sum + val, 0);
        return {
            type,
            dateRange: data[dateRangeKey],
            totalUsage,
            appStats,
            timeStats,
            timeLabels,
            timeDimension,
            rawData: data
        };
    };

    const transformWeeklyData = (data, eyetimeMode = false) =>
        transformPeriodData(data, 'weekly', 'weekRange', 'day', eyetimeMode);

    const transformMonthlyData = (data, eyetimeMode = false) =>
        transformPeriodData(data, 'monthly', 'monthRange', 'week', eyetimeMode);

    const fetchStats = async (deviceId, options = {}) => {
        const { type = 'daily', offset = 0, date = null, mode = 'single' } = options;
        switch (type) {
            case 'daily':
                await fetchDailyStats(deviceId, date, mode);
                break;
            case 'weekly':
                await fetchWeeklyStats(deviceId, offset, mode);
                break;
            case 'monthly':
                await fetchMonthlyStats(deviceId, offset, mode);
                break;
            default:
                error.value = '未知的统计类型';
        }
    };

    return { stats, error, loading, fetchStats };
}
