import createAxios from "@/http/axios";

export const jsonpath_test = (jsonpath: string, data: any) => {
  return createAxios({
    url: `/management/tools/jsonpath_test`,
    method: 'POST',
    data: {
      "jsonpath": jsonpath,
      "inputs": data
    }
  })
};


export const tera_test = (template: string, data: any) => {
  return createAxios({
    url: `/management/tools/tera_test`,
    method: 'POST',
    data: {
      "template": template,
      "inputs": data
    }
  })
};

export const rhai_test = (script: string, ret_type: string, data: any) => {
  return createAxios({
    url: `/management/tools/rhai_test`,
    method: 'POST',
    data: {
      "script": script,
      "return_type": ret_type,
      "inputs": data
    }
  })
};

export const common_test = (script: string, cmd: string, data: any) => {
  return createAxios({
    url: `/management/tools/common_test`,
    method: 'POST',
    data: {
      "script": script,
      "command": cmd,
      "inputs": data
    }
  })
};

