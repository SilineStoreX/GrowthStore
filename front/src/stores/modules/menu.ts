import { defineStore } from "pinia";
import { MenuState } from "@/stores/interface";
import { getFlatArr, getAllBreadcrumbList } from "@/utils/utils";
import { getMenuListApi } from "@/http/modules/login";

export const MenuStore = defineStore({
  id: "MenuState",
  state: (): MenuState => ({
    // 菜单权限列表
    menuList: [],
    routeName: "",
    keepAliveName: [],
  }),
  getters: {
    authMenuList: (state) => state.menuList,
    // 菜单列表扁平化后的数组
    flatMenuList: (state) => getFlatArr(state.menuList),
    // 所有面包屑导航列表
    breadcrumbList: (state) => getAllBreadcrumbList(state.menuList),
  },
  actions: {
    async getMenuList() {
      const dataList = await getMenuListApi();
      this.menuList = dataList;
    },
    setRouteName(name: string) {
      this.routeName = name;
    },
    async addKeepAliveName(name: string) {
      !this.keepAliveName.includes(name) && this.keepAliveName.push(name);
    },
    async removeKeepAliveName(name: string) {
      this.keepAliveName = this.keepAliveName.filter((item) => item !== name);
    },
    async setKeepAliveName(keepAliveName: string[] = []) {
      this.keepAliveName = keepAliveName;
    },
  },
});
