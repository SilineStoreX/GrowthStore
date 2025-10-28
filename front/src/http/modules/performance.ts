import http from "@/http";


export const version_info_get = () => {
  return http.get<any>("/api/version_info");
};

// 获取用户信息
export const performance_get = () => {
  return http.get<any>("/management/performance/get");
};

export const performance_summary = () => {
  return http.get<any>("/management/performance/summary");
};

export const performance_tasks = (dt) => {
  return http.post<any>("/management/performance/tasks", dt);
};

export const performance_oneshots = (dt) => {
  return http.post<any>("/management/performance/tasks", dt);
};

export const performance_logs = () => {
  return http.get<any>("/management/performance/logs");
}

export const download_logs = (file) => {
  return http.get<any>("/management/performance/downloadlogs?file=" + file);
}
