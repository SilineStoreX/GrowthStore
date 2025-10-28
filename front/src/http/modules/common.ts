import createAxios from "@/http/axios_api";
// import createAxios from "@/http/axios";
import createAxiosDirect from "@/http/axios_direct";


export const call_invoke_api = (invk_uri: string, frg: string, method: string, params: any) => {
  var url = "/management/execute/option"
  if (frg.indexOf("paged") >= 0) {
    url = "/management/execute/paged"
  } else if (frg.indexOf("search") >= 0 || frg.indexOf("query") >= 0 || frg.indexOf("list") >= 0) {
    url = "/management/execute/list"
  }

  var data = {
    uri: invk_uri,
    params
  }

  return createAxios({
      url: url,
      method: 'POST',
      data
  })
};


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