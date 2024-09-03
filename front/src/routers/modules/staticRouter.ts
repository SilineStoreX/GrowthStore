import { HOME_URL, LOGIN_URL } from "@/config/config";
import { RouteRecordRaw } from "vue-router";

export const staticRouter: RouteRecordRaw[] = [
  {
    path: "/",
    redirect: HOME_URL,
  },
  {
    path: LOGIN_URL,
    name: "login",
    component: () => import("@/views/Login/index.vue"),
    meta: {
      title: "登录",
    },
  },
  {
    path: "/layout",
    name: "layout",
    component: () => import("@/layouts/index.vue"),
    redirect: HOME_URL,
    children: [],
  },
];

export const errorRouter: RouteRecordRaw[] = [
  {
    path: "/500",
    name: "500",
    component: () => import("@/views/Error/500.vue"),
    meta: {
      title: "500页面",
    },
  },
  {
    path: "/404",
    name: "404", // 页面刷新或登录跳转动态路由会有404的问题
    component: () => import("@/views/Error/404.vue"),
    meta: {
      title: "404页面",
    },
  },
];
