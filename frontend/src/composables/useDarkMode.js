import { ref, watch, onMounted } from 'vue';

/**
 * 暗黑模式管理
 * 支持三种模式：'light' | 'dark' | 'auto'
 * @returns {Object} 暗黑模式状态和切换方法
 */
export function useDarkMode() {
    const isDark = ref(false);
    // 模式：'light' | 'dark' | 'auto'
    const mode = ref('auto');

    const getSystemPreference = () => {
        return window.matchMedia('(prefers-color-scheme: dark)').matches;
    };

    const initDarkMode = () => {
        const stored = localStorage.getItem('themeMode');
        if (stored && ['light', 'dark', 'auto'].includes(stored)) {
            mode.value = stored;
        } else {
            mode.value = 'auto';
        }
        updateDarkMode();
    };

    const updateDarkMode = () => {
        if (mode.value === 'auto') {
            isDark.value = getSystemPreference();
        } else {
            isDark.value = mode.value === 'dark';
        }
        applyDarkMode();
    };

    const applyDarkMode = () => {
        if (isDark.value) {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
    };

    const toggleDarkMode = () => {
        if (mode.value === 'light') {
            mode.value = 'auto';
        } else if (mode.value === 'auto') {
            mode.value = 'dark';
        } else {
            mode.value = 'light';
        }
    };

    const setAutoMode = () => {
        mode.value = 'auto';
    };

    watch(mode, (newValue) => {
        localStorage.setItem('themeMode', newValue);
        updateDarkMode();
    });

    onMounted(() => {
        initDarkMode();
        const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
        const handleChange = () => {
            if (mode.value === 'auto') {
                updateDarkMode();
            }
        };
        mediaQuery.addEventListener('change', handleChange);
        return () => {
            mediaQuery.removeEventListener('change', handleChange);
        };
    });

    return {
        isDark,
        mode,
        toggleDarkMode,
        setAutoMode
    };
}
