<template>
    <div class="login-container">
      <div class="login-left">
        <el-collapse v-model="activeNames">
            <el-collapse-item title="任务提交" name="signin" class="opblock-post">
              <template #title>
                <div class="opblock-summary-block">
                  <span class="opblock-summary-method"><el-icon><Avatar /></el-icon></span>
                </div>
                <div class="opblock-summary-path-description-wrapper">
                  <span class="opblock-summary-path">
                    <a class="nostyle">
                      <span>任务提交</span>
                    </a>
                  </span>
                  <div class="opblock-summary-description"> </div>
                </div>
              </template>
                <div class="form">
                    <el-form ref="loginFormRef" :rules="rules" :model="loginForm" size="default" label-position="top">
                        <el-form-item prop="uri" label="被执行任务的URI">
                            <el-input
                                v-model.trim="loginForm.uri"
                                placeholder="输入被执行任务的URI，必填"
                            />
                        </el-form-item>
                        <el-form-item prop="return_type" label="任务结果返回类型">
                            <el-select v-model.trim="loginForm.return_type">
                                <el-option value="Option">Option</el-option>
                                <el-option value="List">List</el-option>
                                <el-option value="Page">Page</el-option>
                            </el-select>
                        </el-form-item>
                        <el-form-item prop="params" label="执行任务所需要的参数，JSON格式表示">
                            <el-input
                                v-model.trim="loginForm.params"
                                type="textarea"
                                :rows="8"
                                placeholder="输入执行任务所需要的参数，JSON格式表示"
                            />
                        </el-form-item>
                        <el-form-item prop="namespace" label="执行任务绑定的命名空间，该命名空间下须指定Redis配置">
                            <el-input
                                v-model.trim="loginForm.namespace"
                                placeholder="执行任务绑定的命名空间"
                            />
                        </el-form-item>
                        <el-form-item prop="callback" label="执行任务完成后的回调URL，将以POST方式进行回调，可选，不填，则不执行回调">
                            <el-input
                                v-model.trim="loginForm.callback"
                                placeholder="执行任务完成后的回调URL，将以POST方式进行回调"
                            />
                        </el-form-item>
                        <el-form-item prop="uuid" label="任务标识ID，UUID，可选，如不填，则自动生成">
                            <el-input v-model.trim="loginForm.uuid" placeholder="任务标识ID，UUID">
                                <template #append>
                                    <span plain @click="generateUUID" class="button">生成</span>
                                </template>
                            </el-input>
                        </el-form-item>
                        <el-form-item>
                            <el-button
                                type="primary"
                                @click="onSubmitTask(loginFormRef)"
                                :loading="loading"
                                >提交任务</el-button
                            >
                        </el-form-item>
                    </el-form>
                </div>
            </el-collapse-item>
            <el-collapse-item title="任务查询" name="info" class="opblock-get">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><User /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>任务查询</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>
                <div class="form">
                    <el-form ref="queryFormRef" :rules="rules" :model="queryForm" size="default" label-position="top">
                        <el-form-item prop="namespace" label="执行任务绑定的命名空间，该命名空间下须指定Redis配置">
                            <el-input
                                v-model.trim="queryForm.namespace"
                                placeholder="执行任务绑定的命名空间"
                            />
                        </el-form-item>
                        <el-form-item prop="uuid" label="任务标识ID，UUID，提交任务时回传的UUID">
                            <el-input v-model.trim="queryForm.uuid" placeholder="任务标识ID，UUID" />
                        </el-form-item>
                        <el-form-item>
                            <el-button
                                type="primary"
                                @click="onQueryTask(queryFormRef)"
                                :loading="loading"
                                >查询任务结果</el-button
                            >
                        </el-form-item>
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
  import { authorization_get } from "@/http/modules/management";
  import { ElMessage, ElNotification } from "element-plus";
  import { useRouter } from "vue-router";
    import { GlobalStore } from "@/stores";
  import type { FormInstance, FormRules } from "element-plus";
  import { get_javascript_axio_code_md, get_java_code_md, get_curl_code_md, get_javascript_packaged_code_md, get_rhai_code_md } from "@/utils/codetemplate";
  import { MdPreview, MdCatalog } from 'md-editor-v3';
  import 'md-editor-v3/lib/preview.css';
  import { v4 as uuidv4 } from 'uuid';

  const authconf = ref<any>({})
  
  const getAuthorization = () => {
    authorization_get().then((res: any) => {
      authconf.value = res.data
      refresh_auth_org() 
    }).catch((ex: any) => {
      console.log(ex)
    })
  }

  const javascript_code = ref('');
  const java_code = ref('');
  const curl_code = ref('');
  const rhai_code = ref('')
  const packed_code = ref('');
  const logout_code = ref('### 退出登录可以直接在前端清除Token\n```javascript\nglobalStore.setApiToken(null);\n```')
  const codeimage_code = ref('```javascript\ncall_api("/api/auth/code_image", "GET", {}).then(res => {\nloginform.value.captcha_id = res.data.code_id\ndemoCodeUrl.value = \'data:image/png;base64,\' + res.data.image_url\n})\n```');
  
  const activeNames = ref<string>();
  const jsonbody = ref<any>({})
  const globalStore = GlobalStore();
  const router = useRouter();
  // form表单规则校验
  const activeRequest = ref<string>("javascript");
  const loginFormRef = ref();
  const loginForm = reactive<any>({ });
  const queryFormRef = ref();
  const queryForm = reactive<any>({ });
  const auth_org = ref(false);

  const refresh_auth_org = () => {
    auth_org.value = authconf.value && authconf.value.enable_organization
    rules.organization[0].required = auth_org.value
    cpwdrules.organization[0].required = auth_org.value
  }

  const rules = reactive<FormRules>({
    organization: [{ required: auth_org.value, message: "Please input Organization" }],
    username: [{ required: true, message: "Please input Account" }],
    credential: [{ required: true, message: "Please input Password" }],
  });
  const cpwdrules = reactive<FormRules>({
    organization: [{ required: auth_org.value, message: "Please input Organization" }],
    username: [{ required: true, message: "Please input Account" }],
    credential: [{ required: true, message: "Please input old Password" }],
    new_credential: [{ required: true, message: "Please input new Password" }],    
  });
  const loading = ref(false);

  onMounted(() => {
  });
  
  function generateUUID() {
    loginForm.uuid = uuidv4()
  }



  async function call_api_docuemnt(path: string, inv: any, data: any) {
    javascript_code.value = get_javascript_axio_code_md(path, inv, data);
    java_code.value = get_java_code_md(path, inv, data);
    curl_code.value = get_curl_code_md(path, inv, data);
    packed_code.value = get_javascript_packaged_code_md(path, inv, data);
  }
  
  // 登录
  const onSubmitTask = (formEl: FormInstance | undefined) => {
    if (!formEl) return;
    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
      if (!valid) return;
      loading.value = true;
      try {
        let ct = { ...loginForm, params: JSON.parse(loginForm.params)}
        console.log('call api')
        call_api_docuemnt("/api/execute/tasks", { method: "POST" }, ct);
        const data = await call_api("/api/execute/tasks", "POST", ct);
        console.log(data)
        jsonbody.value = data
        if (data.status !== 200) {
          ElMessage.error(data.message);
          return;
        }
      } finally {
        loading.value = false;
      }
    });
  };

  // 登录
  const onQueryTask = (formEl: FormInstance | undefined) => {
    if (!formEl) return;
    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
      if (!valid) return;
      loading.value = true;
      try {
        let ct = { ...queryForm }
        console.log('call api')
        call_api_docuemnt("/api/execute/query", { method: "POST" }, ct);
        const data = await call_api("/api/execute/query", "POST", ct);
        console.log(data)
        jsonbody.value = data
        if (data.status !== 200) {
          ElMessage.error(data.message);
          return;
        }  
      } finally {
        loading.value = false;
      }
    });
  };

  onActivated(() => {
      console.log("activate")
      getAuthorization();
  })

  onUpdated(() => {
      console.log("onUpdate")
      getAuthorization();
  })  
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  