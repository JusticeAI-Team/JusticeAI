import { resolve } from 'node:path'
import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(({ mode }) => {
  const envDir = resolve(__dirname, '..')
  const env = loadEnv(mode, envDir, '')
  const parsedDevPort = Number.parseInt(env.VITE_DEV_PORT ?? '', 10)
  const devPort = Number.isFinite(parsedDevPort) && parsedDevPort > 0 ? parsedDevPort : 18100

  return {
    envDir,
    plugins: [vue()],
    server: {
      host: env.VITE_DEV_HOST || '127.0.0.1',
      port: devPort,
      proxy: {
        '/api': {
          target: env.VITE_API_PROXY_TARGET || 'http://127.0.0.1:8088',
        },
      },
    },
  }
})
