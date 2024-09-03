import { GlobalStore } from "@/stores";

export function get_javascript_packaged_code_md(path: string, inv: any, data: any) {
    const globalStore = GlobalStore()
    let code = "#### 创建Axios对象的方法，将Authorization加入到请求头中\n"
    code += "```javascript ::close\nimport axios from \"axios\";\n"
    code += "\n"
    code += "function createAxios<Data = any, T = ApiPromise<Data>>(axiosConfig: AxiosRequestConfig, options: Options = {}, loading: LoadingOptions = {}): T {\n"
    code += "const Axios = axios.create({\n"
    code += "    baseURL: getUrl(), // 请填写正确的服务器URL\n"
    code += "    timeout: 1000 * 10,\n"
    code += "    headers: {\n"
    code += "        server: true\n"
    code += "    },\n"
    code += "    responseType: 'json',\n"
    code += "})\n"
    code += "\n"
    code += "// 此处省略不少代码.......\n"
    code += "\n"
    code += "Axios.interceptors.request.use(\n"
    code += "    (config) => {\n"
    code += "        const globalStore = GlobalStore(); // 登录的Token被存放在GlobalStore中\n"
    code += "        const token = globalStore.api_token;\n"
    code += "        if (config.headers && typeof config.headers?.set === \"function\")\n"
    code += "          config.headers.set(\"authorization\", 'Bearer ' + token);\n"
    code += "        return config;\n"
    code += "    },\n"
    code += "    (error) => {\n"
    code += "        return Promise.reject(error)\n"
    code += "    }\n"
    code += ")\n"
    code += "// 此处省略不少代码.......\n"
    code += "}\n"
    code += "export default createAxios\n"
    code += "```"
    code += "\n\n#### 下面的建立Call API的公共方法\n"
    code += "```javascript ::close\n"
    code += "import createAxios from \"@/http/axios\";\n\n"
    code += "export const call_api = (url: string, method: string, data: any) => {\n"
    code += "  return createAxios({\n"
    code += "    url: url,\n"
    code += "    method: method,\n"
    code += "    data: data\n"
    code += "  })\n"
    code += "};\n\n\n"
    code += "```"
    code += "\n\n#### 下面的代码是在项目中使用\n"
    code += "```javascript ::open\n"
    code += "call_api('" + path + "', '" + inv.method + "', data).then(res => {\n"
    code += "   if (res.status === 0 || res.status === 200) {\n"
    code += "      businuess.value = res.data\n"
    code += "   } else {\n"
    code += "       ElMessage.warning(\n"
    code += "           \"接口返回信息，\" + res.message\n"
    code += "       )\n"
    code += "   }\n"
    code += "}).catch((ex) => {\n"
    code += "     ElMessage.warning(\n"
    code += "       \"接口调用失败，\" + ex\n"
    code += "     )\n"
    code += "})\n"
    code += "```"    
    return code
}

export function get_javascript_axio_code_md(path: string, inv: any, data: any) {
    const globalStore = GlobalStore()
    let code = "```javascript\nimport axios from \"axios\";\n"
    code += "\n"
    code += "const options = {\n"
    code += "   method: '" + inv.method + "',\n"
    code += "   url: '" + path + "',\n"
    if (globalStore.api_token && globalStore.api_token !== ''){
        code += "   headers: {\n"
        code += "        Authorization: 'Bearer " + globalStore.api_token + "',\n"
        code += "       'content-type': 'application/json'\n"
        code += "   },\n"
    }
    if (inv.method ==="get" || inv.method === 'GET' || inv.method === 'DELETE' || inv.method === 'delete') {
        code += ""
    } else {
        code += "   data: " + JSON.stringify(data, null, 2)
    }
    code += "};\n"
    code += "\n"
    code += "axios.request(options).then(function (response) {\n"
    code += "   console.log(response.data);\n"
    code += "}).catch(function (error) {\n"
    code += "   console.error(error);\n"
    code += "});\n```"
    return code
}

export function get_java_code_md(path: string, inv: any, data: any) {
    const globalStore = GlobalStore()

    let code = "```java\n" 
    code += "OkHttpClient client = new OkHttpClient();\n"
    code += "\n";
    if (inv.method === 'POST' || inv.method === 'PUT') {
        code += "MediaType mediaType = MediaType.parse(\"application/json\");\n"
        code += "RequestBody body = RequestBody.create(mediaType, " + JSON.stringify(JSON.stringify(data, null, 2)) + ");\n"
    }
    code += "Request request = new Request.Builder()\n"
    code += ".url(\"" + path + "\")\n"
    if (inv.method === 'POST') {
        code += ".post(body)\n"
    }
    if (inv.method === 'PUT') {
        code += ".put(body)\n"
    }
    if (inv.method === 'GET') {
        code += ".get()\n"
    }
    if (inv.method === 'DELETE') {
        code += ".delete()\n"
    }
    if (globalStore.api_token && globalStore.api_token !== ''){    
        code += ".addHeader(\"Authorization\", \"Bearer " + globalStore.api_token + "\")\n"
    }
    if (inv.method === 'POST' || inv.method === 'PUT') {
        code += ".addHeader(\"content-type\", \"application/json\")\n"
    }
    code += ".build();\n"
    code += "\n"
    code += "Response response = client.newCall(request).execute();\n"

    return code + "```"
}


export function get_curl_code_md(path: string, inv: any, data: any) {
    const globalStore = GlobalStore()
    let code = "```shell\n"
    code += "curl --request " + inv.method +  " \\ \n";
    code += "--url " + path + " \\ \n";
    if (globalStore.api_token && globalStore.api_token !== ''){
        code += "--header 'Authorization: Bearer " + globalStore.api_token + "' \\ \n"
    }
    if (inv.method === 'POST' || inv.method === 'PUT') {
        code += "--header 'content-type: application/json' \\\n"
        code += "--data '" + JSON.stringify(data, null, 2) + "' \n"
    }
    return code + "```"
}

export function get_rhai_code_md(path: string, inv: any, data: any) {
    const globalStore = GlobalStore()
    let code = "#### Rhai代码主要用于在Hook以及Compose方法中，内部调用相应的方法\n"
    code += "```rhai\n"
    code += `let caller = required(\"${inv.schema}://${inv.namespace}/${inv.name}\")\n`;
    code += "// ctx 是内置于Rhai脚本的执行上下文，必须向这些方法传递\n"
    code += "// args是Rhai脚本的接收到的参数，它是一个数组，通常每个是普通对象，第二个是QueryCondition\n"
    code += `caller.${inv.fragement}(ctx, args) \n`;
    return code + "```"
}
