import type { AxiosRequestConfig, Method } from 'axios'
import axios, {AxiosResponse, AxiosError} from 'axios'
import { ElLoading, ElMessage, ElNotification, type LoadingOptions } from 'element-plus'
import { ResultData } from './interface'
import { GlobalStore } from "@/stores";
import { ResultEnum } from '@/enums/httpEnum';
import { LOGIN_URL } from '@/config/config';
import router from '@/routers';
import { checkStatus } from './status';


interface Options {
    // 是否开启取消重复请求, 默认为 true
    CancelDuplicateRequest?: boolean
    // 是否开启loading层效果, 默认为false
    loading?: boolean
    // 是否开启简洁的数据结构响应, 默认为true
    reductDataFormat?: boolean
    // 是否开启接口错误信息展示,默认为true
    showErrorMessage?: boolean
    // 是否开启code不为0时的信息提示, 默认为true
    showCodeMessage?: boolean
    // 是否开启code为0时的信息提示, 默认为false
    showSuccessMessage?: boolean
    // 当前请求使用另外的用户token
    anotherToken?: string
}

/**
 * 根据运行环境获取基础请求URL
 */
export const getUrl = (): string => {
    const value: string = import.meta.env.VITE_API_URL as string
    return value == 'getCurrentDomain' ? window.location.protocol + '//' + window.location.host : value
}

/**
 * 根据运行环境获取基础请求URL的端口
 */
export const getUrlPort = (): string => {
    const url = getUrl()
    return new URL(url).port
}

type ApiPromise<T = any> = Promise<ResultData<T>>

/**
 * 创建`Axios`
 * 默认开启`reductDataFormat(简洁响应)`,返回类型为`ApiPromise`
 * 关闭`reductDataFormat`,返回类型则为`AxiosPromise`
 */
function createAxios<Data = any, T = ApiPromise<Data>>(axiosConfig: AxiosRequestConfig, options: Options = {}, loading: LoadingOptions = {}): T {

    const Axios = axios.create({
        baseURL: getUrl(),
        timeout: 1000 * 30,
        headers: {
            server: true
        },
        responseType: 'json',
    })

    // 合并默认请求选项
    options = Object.assign(
        {
            CancelDuplicateRequest: true, // 是否开启取消重复请求, 默认为 true
            loading: false, // 是否开启loading层效果, 默认为false
            reductDataFormat: true, // 是否开启简洁的数据结构响应, 默认为true
            showErrorMessage: true, // 是否开启接口错误信息展示,默认为true
            showCodeMessage: true, // 是否开启code不为1时的信息提示, 默认为true
            showSuccessMessage: false, // 是否开启code为1时的信息提示, 默认为false
            anotherToken: '', // 当前请求使用另外的用户token
        },
        options
    )

    // 请求拦截
    Axios.interceptors.request.use(
        (config) => {
            // 创建loading实例
            // 自动携带token
            const globalStore = GlobalStore();
            const token = globalStore.token;
            if (config.headers && typeof config.headers?.set === "function")
              config.headers.set("authorization", 'Bearer ' + token);
            return config;
        },
        (error) => {
            return Promise.reject(error)
        }
    )

    Axios.interceptors.response.use(
        (response: AxiosResponse) => {
          const { data } = response;
          const globalStore = GlobalStore();
          // 登陆失效（code == 401）
          if (data.status) {
            if (data.status === 403 || data.status === 401) {
              ElMessage.error(data.message);
              globalStore.setToken("");
              router.replace(LOGIN_URL);
            }
            
            if (data.status != ResultEnum.SUCCESS &&  data.status !== ResultEnum.SUCCESS_ALT) {
              ElMessage.error(data.message);
              return Promise.reject(data);
            }
          }
          return data;
        },
        async (error: AxiosError) => {
          const { response } = error;
          console.log(error)
          const globalStore = GlobalStore();
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

    return Axios(axiosConfig) as T
}

export default createAxios
