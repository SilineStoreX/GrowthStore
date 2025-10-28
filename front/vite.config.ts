import { defineConfig, loadEnv, ConfigEnv, UserConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { createSvgIconsPlugin } from "vite-plugin-svg-icons"
import vueSetupExtend from "vite-plugin-vue-setup-extend";
import { resolve } from "path";
import viteCompression from 'vite-plugin-compression';
import { viteCommonjs } from '@originjs/vite-plugin-commonjs'
import wasm from "vite-plugin-wasm"; 

const pathSrc = resolve(__dirname, "src")
// https://vitejs.dev/config/
export default defineConfig(({ mode }: ConfigEnv): UserConfig => {
  console.log(mode)
  const env = loadEnv(mode, process.cwd(), '');
  return {
    build: {
      target: ["chrome89", "edge89", "esnext", "firefox89", "safari15"]
    },
    css: {
      preprocessorOptions: {
        scss: {
          api: 'modern-compiler'
        }
      }
    },
    plugins: [vue(), vueSetupExtend(), wasm(), viteCommonjs(),
      viteCompression({
        ext: '.gz', // 生成文件的后缀名
        algorithm: 'gzip', // 压缩算法
        threshold: 10240, // 只有大小大于该值的资源会被处理，单位是字节
        minratio: 0.8, // 只有压缩率小于这个值的资源会被处理
        deleteOriginalAssets: false, // 是否删除原文件
      }),
      createSvgIconsPlugin({
        // 指定需要缓存的图标文件夹
        iconDirs: [resolve(pathSrc, "assets/icons")],
        // 指定symbolId格式
        symbolId: "icon-[dir]-[name]",
      }),],
    esbuild: {
      pure: true ? ["console.log", "debugger"] : [],
    },
    resolve: {
      alias: {
        "@": resolve(__dirname, "./src"),
        "vue-i18n": "vue-i18n/dist/vue-i18n.cjs.js",
      },
    },
    // base: process.env.NODE_ENV === "production" ? "./" : "/",
    base: "./",
    server: {
      port: 3000,
      host: "0.0.0.0",
      open: true,
      proxy: {
        '/api': {
          target: env.VITE_PROXY_SERVER,
          changeOrigin: true
        },
        '/management': {
          target: env.VITE_MANGER_PROXY_SERVER,
          changeOrigin: true
        },
      },
    },
  };
});
