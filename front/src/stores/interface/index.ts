/* GlobalState */
export interface GlobalState {
  token: string;
  api_token: string;
  userInfo: userInfo;
  language: string;
  namespaceTree: number;
  themeConfig: themeConfig;
  tabsMenuList: TabsMenuProps[];
}

export interface userInfo {
  id: string;
  username: string;
  avatar: string;
}

export interface themeConfig {
  layout: LayoutType;
  isCollapse: boolean;
}

export interface MenuState {
  routeName: string;
  menuList: Menu.MenuOptions[];
  keepAliveName: string[];
}

export type LayoutType = "row" | "column";

export interface TabsMenuProps {
  icon: string;
  title: string;
  path: string;
  name: string;
  close: boolean;
}
