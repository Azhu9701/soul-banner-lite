import { defineConfig } from 'vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import dotenv from 'dotenv'
import path from 'path'

// Vite 不自动加载 .env.local —— 必须手动调用
dotenv.config({ path: '.env.local' })

export default defineConfig({
  server: {
    port: 3000,
    proxy: {
      '/api/v1': {
        target: 'http://localhost:3096',
        changeOrigin: true,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, '.'),
    },
  },
  plugins: [
    tailwindcss(),
    tanstackStart({
      srcDirectory: '.',
      router: {
        routesDirectory: 'app',
        routeFileIgnorePattern: '(loading|not-found)',
      },
    }),
    viteReact(),
  ],
})
