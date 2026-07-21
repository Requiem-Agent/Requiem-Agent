import path from 'path';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  base: '/Requiem-Agent/',
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
      // S1-08: alias لـ workspace package حتى يعمل بدون pnpm
      '@workspace/api-client-react': path.resolve(import.meta.dirname, 'lib/api-client-react/src/index.ts'),
      '@workspace/api-zod': path.resolve(import.meta.dirname, 'lib/api-zod/src/index.ts'),
    },
  },
  build: {
    outDir: path.resolve(import.meta.dirname, 'dist/public'),
    emptyOutDir: true,
    // S2-04: Code splitting لتقليل bundle size من 1.1MB
    rollupOptions: {
      output: {
        manualChunks: {
          // React core — يتغير نادراً، يُكاش بشكل ممتاز
          'vendor-react': ['react', 'react-dom', 'wouter'],
          // UI components — shadcn/radix (الأكثر استخداماً)
          'vendor-ui': [
            '@radix-ui/react-dialog',
            '@radix-ui/react-dropdown-menu',
            '@radix-ui/react-select',
            '@radix-ui/react-tabs',
            '@radix-ui/react-tooltip',
            '@radix-ui/react-scroll-area',
            '@radix-ui/react-separator',
            '@radix-ui/react-label',
            '@radix-ui/react-slot',
          ],
          // Data & charts — ثقيل، يُعزل
          'vendor-charts': ['recharts'],
          // Utilities
          'vendor-utils': ['clsx', 'tailwind-merge', 'class-variance-authority', 'lucide-react'],
          // TanStack Query
          'vendor-query': ['@tanstack/react-query'],
        },
      },
    },
    // S2-06: حذف console.log/warn في production build تلقائياً
    esbuild: {
      drop: ['console', 'debugger'],
    },
    // تحذير عند تجاوز 600KB بعد التقسيم
    chunkSizeWarningLimit: 600,
  },
});