import { defineConfig, loadEnv, ConfigEnv, UserConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { createSvgIconsPlugin } from "vite-plugin-svg-icons"
import vueSetupExtend from "vite-plugin-vue-setup-extend";
import { resolve } from "path";

const pathSrc = resolve(__dirname, "src")
// https://vitejs.dev/config/
export default defineConfig(({ mode }: ConfigEnv): UserConfig => {
  console.log(mode)
  const env = loadEnv(mode, process.cwd(), '');
  return {
    plugins: [vue(), vueSetupExtend(),       
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
