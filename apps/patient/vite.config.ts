import { defineConfig } from 'vite'
import viteReact from '@vitejs/plugin-react'
import tsConfigPaths from 'vite-tsconfig-paths'
import { tanstackRouter } from '@tanstack/router-plugin/vite'
import path from 'path'

export default defineConfig({
  // Tauri expects a fixed port, exit if unavailable
  clearScreen: false,
  server: {
    port: 3003,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  resolve: {
    alias: {
      // Specific path for CSS (must come first to match before general pattern)
      '@monobase/ui/styles': path.resolve(__dirname, '../../packages/ui/src/styles/globals.css'),
      // General path for all other imports
      '@monobase/ui': path.resolve(__dirname, '../../packages/ui/src')
    }
  },
  plugins: [
    tsConfigPaths({
      ignoreConfigErrors: true
    }),
    tanstackRouter({
      routesDirectory: './src/routes',
      generatedRouteTree: './src/routeTree.gen.ts',
    }),
    viteReact(),
  ],
  build: {
    outDir: './dist',
    emptyOutDir: true,
    // Tauri supports es2021
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari14',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    esbuild: {
      // Remove console.log in production, keep console.error/warn for debugging
      drop: process.env.TAURI_ENV_DEBUG ? [] : ['console.log'],
    },
  },
})
