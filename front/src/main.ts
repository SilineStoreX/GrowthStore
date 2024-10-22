import { createApp } from "vue";
import App from "./App.vue";
// 全局样式
import "@/style/index.scss";
// iconfont
import "@/assets/iconfont/iconfont.css";
// element plus
import ElementPlus from "element-plus";
import * as Icons from "@element-plus/icons-vue";
// 主题 -- 代替 element-plus/dist/index.css
import "@/style/theme.scss";
// router
import router from "@/routers/index";
// vue i18n(国际化)
import I18n from "@/i18n/index";
// pinia(大菠萝) store
import pinia from "@/stores/index";
import VXEUITable from 'vxe-table'
import 'vxe-table/lib/style.css'
import VxeUI from 'vxe-pc-ui'
import 'vxe-pc-ui/lib/style.css'
import "virtual:svg-icons-register"
import SvgIcon from "@/components/SvgIcon/index.vue"
import './userWorker'

const app = createApp(App);
// 注册element Icons组件
Object.keys(Icons).forEach((key) => {
  app.component(key, Icons[key as keyof typeof Icons]);
});

app.component('SvgIcon', SvgIcon)

app.use(router).use(pinia).use(VXEUITable).use(VxeUI).use(I18n).use(ElementPlus, {  }).mount("#app");
