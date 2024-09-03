import http from "@/http";

// 获取用户信息
export const performance_get = () => {
  return http.get<any>("/management/performance/get");
};
