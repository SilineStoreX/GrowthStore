import { NavigationGuardNext, RouteLocationNormalized, createRouter, createWebHashHistory } from "vue-router";
import NProgress from "@/config/nprogress";
import { staticRouter, errorRouter } from "@/routers/modules/staticRouter";
import { initDynamicRouter } from "@/routers/modules/dynamicRouter";
import { MenuStore } from "@/stores/modules/menu";
import { GlobalStore } from "@/stores";
import { LOGIN_URL } from "@/config/config";

/**
 * @description 动态路由参数配置简介 📚
 * @param path ==> 菜单路径
 * @param name ==> 菜单别名
 * @param redirect ==> 重定向地址
 * @param component ==> 视图文件路径
 * @param meta ==> 菜单信息
 * @param meta.icon ==> 菜单图标
 * @param meta.title ==> 菜单标题
 * @param meta.isFull ==> 是否全屏(如：Three)
 * @param meta.isAffix ==> 是否固定tabs(如：首页)
 * @param meta.isKeepAlive ==> 是否缓存
 * */
const router = createRouter({
  history: createWebHashHistory(),
  routes: [...errorRouter, ...staticRouter],
  strict: false,
  scrollBehavior: () => ({ left: 0, top: 0 }),
});

// 路由拦截
router.beforeEach(async (to: RouteLocationNormalized, from: RouteLocationNormalized, next: NavigationGuardNext) => {
  const globalStore = GlobalStore();
  const menuStore = MenuStore();

  // 1.NProgress 开始
  NProgress.start();
  // 2.判断是否访问登陆页 有token就在当前页面，否则重置路由、重置store菜单并放行到登录页
  if (to.path.toLocaleLowerCase() === LOGIN_URL) {
    if (globalStore.token && globalStore.token !== '') {
      next({ path: from.fullPath, replace: true });
      return;
    }
  
    resetRouter();
    next(undefined);
    return;
  }

  // 3.判断是否有token，没有则重定向到LOGIN_URL
  if (!globalStore.token) {
    next({ path: LOGIN_URL, replace: true });
    return;
  }

  // 4.如果没有菜单列表，就重新请求菜单列表并添加动态路由
  menuStore.setRouteName(to.name as string);
  if (!menuStore.authMenuList.length) {
    await initDynamicRouter();
    next({ ...to, replace: true });
    return;
  }

  // 5.正常访问页面
  next(undefined);
});

// 路由跳转结束
router.afterEach(() => {
  NProgress.done();
});

// 路由跳转错误
router.onError((error: { message: any; }) => {
  NProgress.done();
  console.warn("路由错误", error.message);
});

/**
 * @description 重置路由
 * */
export const resetRouter = () => {
  const menuStore = MenuStore();
  menuStore.flatMenuList.forEach((route: any) => {
    const { name } = route;
    if (name && router.hasRoute(name)) router.removeRoute(name);
  });
};

export default router;
