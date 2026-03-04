import { ref } from 'vue';

const getLocalDateString = (date = new Date()) => {
    const offset = date.getTimezoneOffset() * 60000;
    const localDate = new Date(date - offset);
    return localDate.toISOString().split('T')[0];
};

export function useStatsState() {
    const selectedDate = ref(getLocalDateString());
    const statsType = ref('daily');
    const timeOffset = ref(0);
    const stats = ref(null);

    const getDateRangeText = () => {
        if (!stats.value?.dateRange) return '';
        const { start, end } = stats.value.dateRange;
        return start === end ? start : `${start} 至 ${end}`;
    };

    const handleStatsUpdate = (newStats) => {
        stats.value = newStats;
    };

    return {
        selectedDate,
        statsType,
        timeOffset,
        stats,
        getDateRangeText,
        handleStatsUpdate
    };
}
