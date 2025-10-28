import createAxios from "@/http/axios_api";

export const filesystem_root = (ns: string, fsn: string) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/root`,
    method: 'POST',
    data: {}
  })
};

export const filesystem_tree = (ns: string, fsn: string, mth: string, data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/tree`,
    method: 'POST',
    data
  })
};

export const filesystem_list = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/list`,
    method: 'POST',
    data
  })
};

export const filesystem_delete = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/delete`,
    method: 'POST',
    data
  })
};

export const filesystem_info = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/info`,
    method: 'POST',
    data
  })
};

export const filesystem_rec = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/rec`,
    method: 'POST',
    data
  })
};

export const filesystem_get = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/get`,
    method: 'GET',
    params: data
  })
};


export const filesystem_percent = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/percent`,
    method: 'POST',
    data
  })
};

export const filesystem_mkdir = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/mkdir`,
    method: 'POST',
    data
  })
};

export const filesystem_update = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/update`,
    method: 'POST',
    data
  })
};

export const filesystem_move = (ns: string, fsn: string, mth: string,  data: any) => {
  return createAxios({
    url: `/api/filesystem/${ns}/${fsn}/${mth}/move`,
    method: 'POST',
    data
  })
};

