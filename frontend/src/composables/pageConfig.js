// composables/pageConfig.js
import { ref } from 'vue';
import config from '../config.js';

const API_BASE = config.API_BASE;

export function pageConfig() {
    const pageConfigs = ref({
        config: {
            WEB_DEVICE_COUNT: true,
            WEB_COMMENT: true,
            WEB_AI_SUMMARY: true,
            GISCUS_REPO: '',
            GISCUS_REPOID: '',
            GISCUS_CATEGORY: '',
            GISCUS_CATEGORYID: '',
            GISCUS_MAPPING: 'pathname',
            GISCUS_REACTIONSENABLED: true,
            GISCUS_EMITMETADATA: false,
            GISCUS_INPUTPOSITION: 'bottom',
            GISCUS_THEME: 'light',
            GISCUS_LANG: 'zh-CN'
        },
        tzOffset: 8
    });

    const isLoading = ref(false);
    const error = ref(null);

    const fetchFlags = async () => {
        isLoading.value = true;
        error.value = null;
        try {
            const url = `${API_BASE}/pageConfig`;
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            const data = await response.json();
            if (data && data.success) {
                if (data.config) {
                    pageConfigs.value.config = {
                        ...pageConfigs.value.config,
                        ...data.config
                    };
                }
                if (typeof data.tzOffset === 'number') {
                    pageConfigs.value.tzOffset = data.tzOffset;
                }
            }
        } catch (err) {
            console.error('获取页面配置失败:', err);
            error.value = `获取组件启用状态失败: ${err.message}`;
        } finally {
            isLoading.value = false;
        }
    };

    return { pageConfigs, isLoading, error, fetchFlags };
}
