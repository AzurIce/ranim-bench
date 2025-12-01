import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  base: '/ranim-bench/', // Assuming repo name is ranim-bench
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
})
