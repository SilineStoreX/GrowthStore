import axios, {
  AxiosInstance,
  AxiosError,
  AxiosRequestConfig,
  InternalAxiosRequestConfig,
  AxiosResponse,
} from "axios";
import { ResultData } from "@/http/interface";
import { ResultEnum } from "@/enums/httpEnum";
import { checkStatus } from "./status";
import { GlobalStore } from "@/stores";
import { ElMessage } from "element-plus";
import router from "@/routers";
import { LOGIN_URL } from "@/config/config";

const config = {
  // 默认地址请求地址
  baseURL: import.meta.env.VITE_API_URL as string,
  // 设置超时时间
  timeout: ResultEnum.TIMEOUT as number,
  // 跨域时候允许携带凭证
  withCredentials: true,
  responseType: 'json',
};

class RequestHttp {
  service: AxiosInstance;
  public constructor(config: AxiosRequestConfig) {
    // 实例化axios
    this.service = axios.create(config);
    const globalStore = GlobalStore();
    // 请求拦截器
    this.service.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        const globalStore = GlobalStore();
        const token = globalStore.token;
        if (config.headers && typeof config.headers?.set === "function")
          config.headers.set("authorization", 'Bearer ' + token);
        return config;
      },
      (error: AxiosError) => {
        return Promise.reject(error);
      }
    );

    // 响应拦截器
    this.service.interceptors.response.use(
      (response: AxiosResponse) => {
        const { data } = response;
        const globalStore = GlobalStore();
        // 登陆失效（code == 401）
        if (data.code == ResultEnum.OVERDUE) {
          ElMessage.error(data.msg);
          globalStore.setToken("");
          router.replace(LOGIN_URL);
          return Promise.reject(data);
        }
        return data;
      },
      async (error: AxiosError) => {
        const { response } = error;
        console.log(error)
        // 请求超时 && 网络错误单独判断，没有 response
        if (error.message.indexOf("timeout") !== -1)
          ElMessage.error("请求超时！请您稍后重试");
        if (error.message.indexOf("Network Error") !== -1)
          ElMessage.error("网络错误！请您稍后重试");
        // 根据响应的错误状态码，做不同的处理
        if (response) checkStatus(response.status);
        if (response.status === 403 || response.status === 401) {
          console.log('redirect to /login')
          globalStore.setToken("");
          router.replace(LOGIN_URL);
        } 
        // 服务器结果都没有返回(可能服务器错误可能客户端断网)，断网处理:可以跳转到断网页面
        if (!window.navigator.onLine) router.replace("/500");
        return Promise.reject(error);
      }
    );
  }

  // 常用请求方法
  get<T>(url: string, params?: object, _object = {}): Promise<ResultData<T>> {
    return this.service.get(url, { params, ..._object });
  }
  post<T>(url: string, params?: object, _object = {}): Promise<ResultData<T>> {
    return this.service.post(url, params, { data: params });
  }
  put<T>(url: string, params?: object, _object = {}): Promise<ResultData<T>> {
    return this.service.put(url, params, _object);
  }
  delete<T>(url: string, params?: any, _object = {}): Promise<ResultData<T>> {
    return this.service.delete(url, { params, ..._object });
  }
  download(url: string, params?: object, _object = {}): Promise<BlobPart> {
    return this.service.post(url, params, { ..._object, responseType: "blob" });
  }
}

export default new RequestHttp(config as AxiosRequestConfig);
