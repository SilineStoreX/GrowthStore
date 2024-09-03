import { userInfo } from "@/http/interface/index";
import qs from "qs";
import http from "@/http";

// 获取用户信息
export const getUserInfo = () => {
  return http.post<userInfo>("/management/userinfo");
};
