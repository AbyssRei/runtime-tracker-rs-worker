// 前端配置
// API_BASE 使用相对路径，与 Cloudflare Worker 同源部署
const config = {
    dev: {
        API_BASE: import.meta.env.VITE_API_BASE || '/api',
        ADMIN_URL: import.meta.env.VITE_ADMIN_URL || '/admin/login',
        SITE_TITLE: import.meta.env.VITE_SITE_TITLE || 'RunTime Tracker'
    },
    prod: {
        API_BASE: import.meta.env.VITE_API_BASE || '/api',
        ADMIN_URL: import.meta.env.VITE_ADMIN_URL || '/admin/login',
        SITE_TITLE: import.meta.env.VITE_SITE_TITLE || 'RunTime Tracker'
    }
};

export default import.meta.env.PROD ? config.prod : config.dev;
