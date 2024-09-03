import createAxios from "@/http/axios_api";
import createAxiosDirect from "@/http/axios_direct";

/**
 * @name 登录模块
 */

// 用户登录
export const call_api = (url: string, method: string, data: any) => {

  if (method === 'GET' || method === 'DELETE' || method === 'get' || method === 'delete') {
    return createAxios({
      url: url,
      method: method,
      params: data
    })
  } else {
    return createAxios({
      url: url,
      method: method,
      data: data
    })
  }
};

export const call_api_options = (url: string, method: string, data: any, opts: any) => {

  if (method === 'GET' || method === 'DELETE' || method === 'get' || method === 'delete') {
    return createAxios({
      url: url,
      method: method,
      params: data
    }, opts)
  } else {
    return createAxios({
      url: url,
      method: method,
      data: data
    }, opts)
  }
};



export const call_direct = (url: string, method: string, data: any) => {

  if (method === 'GET' || method === 'DELETE' || method === 'get' || method === 'delete') {
    return createAxiosDirect({
      url: url,
      method: method,
      params: data
    })
  } else {
    return createAxiosDirect({
      url: url,
      method: method,
      data: data
    })
  }
};