import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path';


export default defineConfig({
  plugins: [react()],
  define: {
    'process.env': process.env
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },    
  server: {
    port: process.env.DEMO_FRONTEND_PORT || 5173,
    host: true
  }
})

