import http from "@/http";

// 获取用户信息
export const performance_get = () => {
  return http.get<any>("/management/performance/get");
};

export const performance_summary = () => {
  return http.get<any>("/management/performance/summary");
};
