<template>
    <div class="login-container">
      <div class="login-left">
        <div class="header">
          <el-form>
            <el-form-item label="服务器">
              <el-select v-model="server">
                <el-option v-for="s in servers" :key="s" :value="s.url" >{{ s.description ? s.description : '' + '-' + s.url }}</el-option>
              </el-select>
            </el-form-item>
          </el-form>
        </div>        
        <el-collapse v-model="activeNames" accordion @change="onMapInvocation">
            <el-collapse-item v-for="p in paths" :key="p.path" :name="p.fullname" :class="'opblock-' + p.method.toLowerCase() ">
              <template #title>
                <div class="opblock-summary-block">
                  <span class="opblock-summary-method">{{ p.method }}</span>
                </div>
                <div class="opblock-summary-path-description-wrapper">
                  <span class="opblock-summary-path">
                    <a class="nostyle">
                      <span>{{ p.path }} {{ p.detail.summary }}</span>
                    </a>
                  </span>
                </div>
              </template>
              <div class="form">
                <p class="description">{{ p.detail.description }}</p>
                <el-form size="small" label-width="100px" :inline="false">
                  <el-form-item v-for="dp in p.detail.parameters" :key="dp.name" :label="dp.name">
                    <el-input v-model="invocation[dp.name]"></el-input>
                  </el-form-item>
                  <el-form-item v-if="p.detail.requestBody" label="Content-Type">
                    <el-select v-model="invocation.request_content_type">
                      <el-option v-for="vr in contentTypes" :key="vr" :value="vr" :label="vr"></el-option>
                    </el-select>
                  </el-form-item>
                  <el-form-item v-if="p.detail.requestBody && invocation.request_content_type" label="RequestBody">
                    <el-tabs v-model="activateTabs" style="width: 100%">
                      <el-tab-pane label="请求模板" name="first">
                        <el-upload
                          v-if="p.fileupload"
                          class="upload-demo"
                          :action="server + p.path"
                          :headers="get_auth_header()"
                          :on-preview="handlePreview"
                          :on-remove="handleRemove"
                          :before-remove="beforeRemove"
                          multiple
                          :data="{key: 'test'}"
                          :limit="3"
                          :on-exceed="handleExceed"
                          :file-list="fileList"
                        >
                          <el-button size="small" type="primary">点击上传</el-button>
                          <template #tip>
                            <div class="el-upload__tip">只能上传 jpg/png 文件，且不超过 500kb</div>
                          </template>
                        </el-upload>
                        <el-input v-else type="textarea" v-model="invocation.request_body" :rows="8"></el-input>
                      </el-tab-pane>
                      <el-tab-pane label="参数描述" name="second">
                        <JsonViewer :value="p.detail.requestBody.content[invocation.request_content_type] && p.detail.requestBody.content[invocation.request_content_type].schema" :expand-depth="2" copyable boxed />
                      </el-tab-pane>
                    </el-tabs>
                  </el-form-item>
                  <el-form-item style="padding: 10px; width: 100%;">
                    <el-button type="primary" @click="onInvoke" :loading="loading">执行</el-button>
                  </el-form-item>                  
                  <div class="el-form-item el-form-item--small" style="display: block;">
                    <span class="title">Response</span>
                    <table class="response-table">
                      <thead>
                        <tr class="responses-header">
                          <td class="col_header response-col_status">Code</td>
                          <td class="col_header response-col_description">Description</td>
                        </tr>                                                    
                      </thead>
                      <tbody>
                        <tr v-for="(k, v) of p.detail.responses" :key="v">
                          <td class="col">{{ v }}</td>
                          <td class="col">
                            <span class="summary">{{ k.description }}</span>
                            <div v-for="(m, n) of k.content" :key="n" class="ct">
                              <span>Media Type: {{ n }} </span>
                              <JsonViewer :value="m.schema" :expand-depth="2" copyable boxed />
                            </div>
                          </td>
                        </tr>
                        <tr v-for="(k, v) of default_responses" :key="v">
                          <td class="col">{{ v }}</td>
                          <td class="col">
                            <span class="summary">{{ k.description }}</span>
                            <div v-for="(m, n) of k.content" :key="n" class="ct">
                              <span>Media Type: {{ n }} </span>
                              <JsonViewer :value="m.schema" :expand-depth="0" boxed />
                            </div>
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </el-form>
              </div>
            </el-collapse-item>
        </el-collapse>
      </div>
      <div class="login-right">
        <div class="content">
          <el-tabs v-model="activeRequest">
            <el-tab-pane label="JavaScript(Axio)" name="javascript">
              <MdPreview editorId="javascript_id" :modelValue="javascript_code" />
            </el-tab-pane>
            <el-tab-pane label="JavaScript(封装)" name="packed_code">
              <MdPreview editorId="packed_id" :modelValue="packed_code" />
            </el-tab-pane>
            <el-tab-pane label="Java" name="javacode">
              <MdPreview editorId="java_id" :modelValue="java_code" />
            </el-tab-pane>
            <el-tab-pane label="cURL" name="curl">
              <MdPreview editorId="curl_id" :modelValue="curl_code" />
            </el-tab-pane>
            <el-tab-pane label="Rhai" name="rhai">
              <MdPreview editorId="rhai_id" :modelValue="rhai_code" />
            </el-tab-pane>
          </el-tabs>
          <span class="title">接口调用响应</span>
          <JsonViewer :value="jsonbody" :expand-depth="10" copyable boxed sort></JsonViewer>
        </div>
      </div>
    </div>
  </template>
  
  <script lang="ts" setup>
  import JsonViewer from 'vue-json-viewer'
  import { ref, reactive, onMounted, onActivated, onUpdated } from "vue";
  import { call_api } from "@/http/modules/common";
   import { ElMessage, ElNotification } from "element-plus";
  import { useRoute } from "vue-router";
  import { GlobalStore } from "@/stores";
  import { get_javascript_axio_code_md, get_java_code_md, get_curl_code_md, get_javascript_packaged_code_md, get_rhai_code_md } from "@/utils/codetemplate";
  import { MdPreview, MdCatalog } from 'md-editor-v3';
// preview.css相比style.css少了编辑器那部分样式
  import 'md-editor-v3/lib/preview.css';
  import { format_date } from "@/utils/datetime"

  const javascript_code = ref('');
  const java_code = ref('');
  const curl_code = ref('');
  const packed_code = ref('');
  const rhai_code = ref('');

  const scrollElement = document.documentElement;
  const activateTabs = ref<string>("first");
  const activeNames = ref<string>();
  const activeRequest = ref<string>("javascript");
  const jsonbody = ref<any>({})
  const globalStore = GlobalStore();
  const route = useRoute();
  const loading = ref(false);
  const invocation = ref<any>({});

  const server = ref<string>("");
  const servers = ref<Array<any>>([]);
  const paths = ref<Array<any>>([]);

  const contentTypes = ref<Array<any>>([]);

  const default_resp = {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "data": {
                        "type": "object"
                      },
                      "message": {
                        "type": "string"
                      },
                      "status": {
                        "type": "integer"
                      },
                      "timestamp": {
                        "type": "integer"
                      }
                    }
                  }
                }
              };
  const default_responses = 
    {
      "400": {
        "description": "请求的RequestBody无法被解析成为JSON对象。status=400",
        "content": default_resp
      },
      "401": {
        "description": "Unauthorized，没有正确登录。status=401。",
        "content": default_resp
      },
      "403": {
        "description": "Permission Denined，用户无权限访问该资源。status=403。",
        "content": default_resp
      },
      "404": {
        "description": "Not Found，无法找到所请求的资源。status=404。",
        "content": default_resp
      },
      "500": {
        "description": "服务器异常。status=500。",
        "content": default_resp
      },
    };

    const fileList = ref<Array<any>>([])

  function handleRemove(file, fileList) {
    console.log(file, fileList)
  }

  function handlePreview(file) {
        console.log(file)
  }

  function get_auth_header() {
    if (globalStore.api_token) {
      return {
        "Authorization": "Bearer " + globalStore.api_token
      }
    } else if (globalStore.token) {
      return {
        "Authorization": "Bearer " + globalStore.token
      }
    } else {
      return {}
    }
  }

  function handleExceed(files, fileList) {
      ElMessage.warning(
        `当前限制选择 3 个文件，本次选择了 ${files.length} 个文件，共选择了 ${
          files.length + fileList.length
        } 个文件`
      )
  }

  function beforeRemove(file, fileList) {
    ElMessage(`确定移除 ${file.name}？`)
    return true
  }

  onMounted(() => {
    // 监听enter事件（调用登录）
    fetch_openapi_json();
  });


  onActivated(() => {
      console.log("activate")
      fetch_openapi_json();
  })

  onUpdated(() => {
      console.log("onUpdate")
      // fetch_openapi_json();
  })

  function fetch_openapi_json() {
    var ns = route.query.ns as string
    var spns = ns.split(":")
    let protocol = spns[1] === "_object" ? "object" : (spns[1] === "_query" ? "query": spns[1]);
    let namespace = spns[0] 
    let cpname = spns.length > 2 ? spns[2] : null
    var filter_ = "/api/" + protocol + "/" + namespace + "/" + ( cpname ? cpname + '/' : '')
    var filter_passoff = "/api/passoff/" + protocol + "/" + namespace + "/" + ( cpname ? cpname + '/' : '')

    call_api(`/api/metadata/${namespace}/api-doc/openapi.json`, "GET", {}).then((res: any) => {
      servers.value = res.servers
      var ps = Array<any>()
      for (var key in res.paths) {
        if(key.startsWith(filter_) || key.startsWith(filter_passoff)) {
          let d = res.paths[key]
          for (var n in d) {
            var fullname = n + '_' + key
            var tags = d[n].tags
            ps.push({
              path: key,
              method: n.toUpperCase(),
              fullname: fullname,
              fileupload: tags ? tags.indexOf('fileupload') >= 0: false,
              detail: d[n]
            })
          }
        }
      }
      paths.value = ps
    }).catch(ex => {
      console.log(ex)
    })
  }
  
  function onInvoke(dp: any) {
    console.log("Server: " + server.value, dp, invocation.value)
    var inv = invocation.value
    var p = invocation.value.detail
    var path = inv.path
    if (p.parameters) {
      for (var param of p.parameters) {
        let pval = inv[param.name]
        path = path.replace("{" + param.name + "}", pval)
      }
    }
    console.log('Request Path: ' + path)
    let cp = {}
    try {
      if (inv.request_body && inv.request_body !== '') {
        cp = JSON.parse(inv.request_body)
      }
    } catch(e: any) {
      console.log(e)
      ElMessage.error("请求体内容被解释成JSON对象时错误。" + e)
    }
    javascript_code.value = get_javascript_axio_code_md(path, inv, cp);
    java_code.value = get_java_code_md(path, inv, cp);
    curl_code.value = get_curl_code_md(path, inv, cp);
    packed_code.value = get_javascript_packaged_code_md(path, inv, cp);
    rhai_code.value = get_rhai_code_md(path, inv, cp);

    call_api(path, inv.method, cp).then(res => {
      jsonbody.value = res
    })
  }

  function onMapInvocation(dp: any) {
    if (!dp) return;
    var fullname = dp;
    var pathx = paths.value
    
    pathx.filter(p => p.fullname === fullname).forEach((p: any) => {
      console.log('map invocation', p)
      var invk: any = {
        method: p.method,
        path: p.path,
        fileupload:  p.fileupload,
      };
      let cts = Array<string>()
      if (p.detail.requestBody && p.detail.requestBody.content) {
        for (var ct in p.detail.requestBody.content) {
          cts.push(ct)
          invk.request_content_type = ct
          invk.request_body =  schema_to_default_value(p.detail.requestBody.content[ct].schema)
        }
      }

      // parse path
      if (p.path) {
        var sppath = p.path.split('/')
        if (sppath.length >= 6) {
          if (sppath[2] === 'passoff') {
            invk.schema = sppath[3]
            invk.namespace = sppath[4]
            invk.name = sppath[5]
            invk.fragement = sppath[6]
          } else {
            invk.schema = sppath[2]
            invk.namespace = sppath[3]
            invk.name = sppath[4]
            invk.fragement = sppath[5]
          }
        }
      }
      invk.detail = p.detail

      contentTypes.value = cts
      console.log(cts)
      invocation.value = invk
    })
  }

  function to_object(schema: any) {
    if (!schema) return;
    if (schema.type === 'object') {
      if (schema.properties) {
        var obj = {}
        for (let kk in schema.properties) {
          console.log(kk);
          obj[kk] = to_object(schema.properties[kk])
        }
        return obj
      } else {
        if (schema.format === 'date' || schema.format === 'date-time') {
          return format_date(new Date(), "%Y-%m-%d %H:%M:%S.%s")
        } else {
          return {}
        }
      }
    } else if (schema.type === 'string') {
      return "string";
    } else if (schema.type === 'number' || schema.type === 'integer') {
      return 0;
    } else if (schema.type === 'array') {
      var tc = to_object(schema.items);
      if(tc) {
        return [tc];
      } else {
        return [];
      }
    } else if (schema.type === 'boolean') {
      return false;
    } else {
      return null;
    }
  }

  function schema_to_default_value(schema: any) {
    if (schema.type === 'object') {
      if (schema.properties) {
        var obj = {}
        for (var kk in schema.properties) {
          obj[kk] = to_object(schema.properties[kk])
        }
        return JSON.stringify(obj, null, 4);
      } else {
        if (schema.format === 'date' || schema.format === 'date-time') {
          return JSON.stringify(format_date(new Date(), "%Y-%m-%d %H:%M:%S.%s"))
        } else {
          return JSON.stringify({})
        }
      }
    } else if (schema.type === 'string') {
      return "string";
    } else if (schema.type === 'number') {
      return 0;
    } else if (schema.type === 'array') {
      var tc = to_object(schema.items);
      if(tc) {
        return JSON.stringify([tc], null, 2);
      } else {
        return JSON.stringify([]);
      }
    } else if (schema.type === 'boolean') {
      return JSON.stringify(false);
    } else {
      return JSON.stringify(null);
    }
  }

  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  