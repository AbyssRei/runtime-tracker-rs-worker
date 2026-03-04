import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
    plugins: [
        vue(),
        tailwindcss()
    ],
    build: {
        // 输出到 worker 项目的 dist 目录 (Cloudflare Workers Static Assets)
        outDir: '../dist',
        emptyOutDir: true
    }
})
