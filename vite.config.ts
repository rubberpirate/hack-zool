import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), 'ZSPLIT_');
  const backendUrl = env.ZSPLIT_API_URL || 'http://127.0.0.1:8080';

  return {
    plugins: [tailwindcss(), sveltekit({ adapter: adapter(), preprocess: vitePreprocess() })],
    server: {
      host: '0.0.0.0',
      port: 5173,
      proxy: {
        '/api': {
          target: backendUrl,
          changeOrigin: true,
        },
        '/healthz': {
          target: backendUrl,
          changeOrigin: true,
        },
      },
    },
  };
});
