import { defineStore, createPinia } from "pinia";
import { GlobalState, userInfo, themeConfig, TabsMenuProps } from "./interface";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";
import piniaPersistConfig from "@/config/piniaPersist";
import router from "@/routers/index";

export const GlobalStore = defineStore({
  id: "GlobalState",
  state: (): GlobalState => ({
    userInfo: {
      id: "",
      username: "",
      avatar: "",
    },
    language: "",
    token: "",
    api_token: "",
    namespaceTree: 0,
    tabsMenuList: [],
    themeConfig: {
      // 布局-- 横向:'column' | 纵向:'row'
      layout: "column",
      // 折叠菜单
      isCollapse: false,
    },
  }),
  getters: {},
  actions: {
    updateLanguage(language: string) {
      this.language = language;
    },
    updateNamespaceTree() {
      this.namespaceTree = new Date().getTime()
    },
    setUserInfo(userInfo: userInfo) {
      this.userInfo = userInfo;
    },
    setThemeConfig(themeConfig: themeConfig) {
      this.themeConfig = themeConfig;
    },
    setToken(token: string) {
      this.token = token;
    },
    setApiToken(token: string) {
      this.api_token = token;
    },
    addTabs(tabItem: TabsMenuProps) {
      if (this.tabsMenuList.every((item) => item.path !== tabItem.path)) {
        this.tabsMenuList.push(tabItem);
      }
    },
    setTabs(tabsMenuList: TabsMenuProps[]) {
      this.tabsMenuList = tabsMenuList;
    },
    removeTabs(tabPath: string, isCurrent: boolean = true) {
      const tabsMenuList = this.tabsMenuList;
      if (isCurrent) {
        tabsMenuList.forEach((item, index) => {
          if (item.path !== tabPath) return;
          const nextTab = tabsMenuList[index + 1] || tabsMenuList[index - 1];
          if (!nextTab) return;
          router.push(nextTab.path);
        });
      }
      this.tabsMenuList = tabsMenuList.filter((item) => item.path !== tabPath);
    },
    setTabsTitle(title: string) {
      const nowFullPath = location.hash.substring(1);
      this.tabsMenuList.forEach((item) => {
        if (item.path == nowFullPath) item.title = title;
      });
    },
    closeMultipleTab(tabsMenuValue?: string) {
      this.tabsMenuList = this.tabsMenuList.filter((item) => {
        return item.path === tabsMenuValue || item.name === 'home';
      });
    },

    closeLeftTab(tabsMenuValue?: string) {
      var cp = false;
      this.tabsMenuList = this.tabsMenuList.filter((item) => {
        console.log(JSON.stringify(item))
        if (item.path === tabsMenuValue) {
          cp = true
        }
        return cp || item.name === 'home';
      });
      router.push(tabsMenuValue);
    },
    closeRightTab(tabsMenuValue?: string) {
      var cp = false;
      var nextcp = false;
      this.tabsMenuList = this.tabsMenuList.filter((item) => {
        if (item.path === tabsMenuValue) {
          cp = true
        }
        let cx = (cp === false || nextcp === false) || item.name === 'home';
        if (cp) {
          nextcp = true
        }
        return cx;
      });
      router.push(tabsMenuValue);
    },
  },
  persist: piniaPersistConfig("GlobalState"),
});

// piniaPersist(持久化)
const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

export default pinia;
