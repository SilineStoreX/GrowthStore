import router from "@/routers/index";
import { MenuStore } from "@/stores/modules/menu";
import { GlobalStore } from "@/stores";
import { ElNotification } from "element-plus";
import { isType } from "@/utils/utils";
import { LOGIN_URL } from "@/config/config";

// 引入 views 文件夹下所有 vue 文件
const modules = import.meta.glob("@/views/**/*.vue");
export const initDynamicRouter = async () => {
  const menuStore = MenuStore();
  const globalStore = GlobalStore();

  try {
    // 1.获取菜单列表
    await menuStore.getMenuList();

    // 2.判断当前用户有没有菜单权限
    if (!menuStore.authMenuList.length) {
      ElNotification({
        title: "无权限访问",
        message: "当前账号无任何菜单权限，请联系系统管理员！",
        type: "warning",
        duration: 3000,
      });
      router.replace(LOGIN_URL);
      return Promise.reject("No permission");
    }

    // 3.添加动态路由
    menuStore.flatMenuList.forEach((item: any) => {
      console.log('flatMenuList', item)
      item.children && delete item.children;
      if (item.component && isType(item.component) == "string") {
        item.component = modules["/src/views" + item.component + ".vue"];
      }
      if (item.meta.isFull) {
        router.addRoute(item);
      } else {
        router.addRoute("layout", item);
      }
    });
  } catch (error) {
    // 当菜单请求出错时，重定向到登陆页
    globalStore.setToken("");
    router.replace(LOGIN_URL);
    return Promise.reject(error);
  }
};
