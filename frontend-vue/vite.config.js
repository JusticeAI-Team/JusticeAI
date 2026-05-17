import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

const lifecycleEvent = process.env.npm_lifecycle_event || ''
const appEntry = process.env.VITE_APP_ENTRY || (lifecycleEvent.includes('mobile') ? 'mobile' : 'desktop')

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  define: {
    'import.meta.env.VITE_APP_ENTRY': JSON.stringify(appEntry)
  },
  server: {
    host: process.env.VITE_DEV_HOST || '127.0.0.1',
    port: Number(process.env.VITE_DEV_PORT || (appEntry === 'mobile' ? 18101 : 18100)),
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY_TARGET || 'http://127.0.0.1:8088',
        changeOrigin: true
      }
    }
  }
})
