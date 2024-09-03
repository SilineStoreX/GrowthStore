import { Login } from "@/http/interface/index";
import qs from "qs";
import http from "@/http";
import menuList from "@/json/menu.json";
import createAxios from "@/http/axios";

/**
 * @name 登录模块
 */

// 用户登录
export const loginApi = (params: Login.ReqLoginForm) => {
  return createAxios({
    url: "/management/login",
    method: 'POST',
    data: params
  })
};

// 获取菜单列表
export const getMenuListApi = () => {
  return menuList;
};


export const modifyPassword = (data: any) => {
  return createAxios({
    url: "/management/changepwd",
    method: 'POST',
    data: data
  })
}
