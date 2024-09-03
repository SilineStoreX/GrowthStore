import { userInfo } from "@/http/interface/index";
import http from "@/http";
import createAxios from "@/http/axios";

// 获取用户信息
export const fetchNamespaces = () => {
  return http.get<any>("/management/fetch/namespaces");
};

// 获取用户信息
export const fetchConfig = (ns: string) => {
  return http.get<any>("/management/fetch/config?ns=" + ns);
};

// 获取Namespace下的所有表
export const probeTables = (ns: string, sch: string) => {
  return http.get<any>("/management/probe/schema?ns=" + ns + '&schema=' + sch);
};

export const generate = (ns: string, sch: string, rule: string, tables: Array<any>) => {
  return createAxios({
    url: "/management/generate?ns=" + ns + '&schema=' + sch + '&rule=' + rule,
    method: 'POST',
    data: tables
  })
};

export const update = (ns: string, typ_: string, obj: any) => {
  return createAxios({
    url: "/management/update?ns=" + ns + '&type=' + typ_,
    method: 'POST',
    data: obj
  })
};

export const remove = (ns: string, typ_: string, objarr: Array<string>) => {
  return createAxios({
    url: "/management/delete?ns=" + ns + '&type=' + typ_,
    method: 'POST',
    data: objarr
  })
};

export const configDelete = (ns: string) => {
  return createAxios({
    url: "/management/delete?ns=" + ns + '&type=namespace',
    method: 'POST',
    data: [ns]
  })
};


export const authorization_get = () => {
  return createAxios({
    url: "/management/authorization",
    method: 'GET'
  })
};

export const authorization_post = (data: any) => {
  return createAxios({
    url: "/management/authorization",
    method: 'POST',
    data
  })
};

export const authorize_roles_get = () => {
  return createAxios({
    url: "/management/authorize/roles",
    method: 'GET'
  })
};

export const plugin_list = () => {
  return createAxios({
    url: "/management/plugin/list",
    method: 'GET'
  })
};

export const metadata_get = (scheme: string) => {
  return createAxios({
    url: `/management/metadata/${scheme}/schema.json`,
    method: 'GET'
  })
};

export const config_get = (scheme: string, ns: string, name: string) => {
  return createAxios({
    url: `/management/config/get?schema=${scheme}&ns=${ns}&name=${name}`,
    method: 'GET'
  })
};

export const config_save = (scheme: string, ns: string, name: string, data: any) => {
  return createAxios({
    url: `/management/config/save?schema=${scheme}&ns=${ns}&name=${name}`,
    method: 'POST',
    data
  })
};

export const config_create = (scheme: string, ns: string, data: any) => {
  return createAxios({
    url: `/management/config/create?type=${scheme}&ns=${ns}`,
    method: 'POST',
    data
  })
};

export const lang_list = () => {
  return createAxios({
    url: `/management/lang/list`,
    method: 'GET',
  })
};

export const redis_keys = (ns: string, key_prefix: string) => {
  return createAxios({
    url: `/management/${ns}/redis/keys?key=${key_prefix}`,
    method: 'GET' 
  })
}

export const archive = (ns: string) => {
  return createAxios({
    url: `/management/config/archive?ns=${ns}`,
    method: 'GET',
    responseType: 'blob'
  })
}

export const redis_del = (ns: string, key: string) => {
  return createAxios({
    url: `/management/${ns}/redis/delexp?key=${key}`,
    method: 'POST' 
  })
}

export const redis_get = (ns: string, key: string) => {
  return createAxios({
    url: `/management/${ns}/redis/get?key=${key}`,
    method: 'GET' 
  })
}

export const redis_flushall = (ns: string) => {
  return createAxios({
    url: `/management/${ns}/redis/flushall`,
    method: 'POST' 
  })
}

export const fetch_pluginnames = (ns: string, p: string) => {
  return createAxios({
    url: `/management/fetch/pluginnames?ns=${ns}&protocol=${p}`,
    method: 'GET' 
  })
}

export const common_manage = (protocol: string, ns: string, name: string, method: string, data: any) => {
  return createAxios({
    url: `/management/common/${protocol}/${ns}/${name}/${method}`,
    method: 'POST',
    data
  })
}
