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
  },
});