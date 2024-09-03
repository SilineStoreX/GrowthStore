<template>
    <div class="login-container">
      <div class="login-left">
        <el-collapse v-model="activeNames">
            <el-collapse-item title="登录" name="signin" class="opblock-post">
              <template #title>
                <div class="opblock-summary-block">
                  <span class="opblock-summary-method"><el-icon><Avatar /></el-icon></span>
                </div>
                <div class="opblock-summary-path-description-wrapper">
                  <span class="opblock-summary-path">
                    <a class="nostyle">
                      <span>登录</span>
                    </a>
                  </span>
                  <div class="opblock-summary-description"> </div>
                </div>
              </template>
                <div class="form">
                    <el-form ref="loginFormRef" :rules="rules" :model="loginForm">
                        <el-form-item v-if="authconf.enable_organization" prop="organization">
                            <el-input
                                v-model.trim="loginForm.organization"
                                placeholder="输入所在的组织代码"
                            />
                        </el-form-item>
                        <el-form-item prop="username">
                            <el-input v-model.trim="loginForm.username" placeholder="用户帐号" />
                        </el-form-item>
                        <el-form-item prop="credential">
                            <el-input
                                type="password"
                                show-password
                                v-model.trim="loginForm.credential"
                                placeholder="密码"
                            />
                        </el-form-item>
                        <el-form-item v-if="authconf.enable_captcha" prop="captcha_code">
                            <el-input
                                v-model.trim="loginForm.captcha_code"
                                placeholder="输入图片验证码"
                                style="width: calc(100% - 100px); padding-right: 20px;"
                            />
                            <img :src="loginCodeUrl" style="height: 40px;" @click="getLoginCode">
                        </el-form-item>
                        <el-form-item>
                            <el-button
                                type="primary"
                                @click="onLogin(loginFormRef)"
                                :loading="loading"
                                >登录</el-button
                            >
                        </el-form-item>
                    </el-form>
                </div>
            </el-collapse-item>
            <el-collapse-item v-if="authconf.enable_api_secure" title="交换Token" name="exchange" class="opblock-post">
              <template #title>
                <div class="opblock-summary-block">
                  <span class="opblock-summary-method"><el-icon><Avatar /></el-icon></span>
                </div>
                <div class="opblock-summary-path-description-wrapper">
                  <span class="opblock-summary-path">
                    <a class="nostyle">
                      <span>使用AppId/AppSecret交换Token</span>
                    </a>
                  </span>
                  <div class="opblock-summary-description"> </div>
                </div>
              </template>
                <div class="form">
                    <el-form ref="loginFormRef" :rules="rules" :model="loginForm">
                        <el-form-item prop="app_id">
                            <el-input v-model.trim="loginForm.app_id" placeholder="AppId" />
                        </el-form-item>
                        <el-form-item prop="app_secret">
                            <el-input
                                type="password"
                                show-password
                                v-model.trim="loginForm.app_secret"
                                placeholder="App Secret"
                            />
                        </el-form-item>
                        <el-form-item>
                            <el-button
                                type="primary"
                                @click="onExchange(loginFormRef)"
                                :loading="loading"
                                >交换Token</el-button
                            >
                        </el-form-item>
                    </el-form>
                </div>
            </el-collapse-item>
            <el-collapse-item title="获取当前登录用户信息" name="info" class="opblock-get">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><User /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>获取当前登录用户信息</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>
                <div class="form">
                    <el-button
                        type="primary"
                        @click="onGetUserInfo"
                        :loading="loading"
                        >获取当前登录用户信息</el-button
                    >
                </div>
            </el-collapse-item>
            <el-collapse-item title="刷新登录用户Token" name="refresh" class="opblock-get">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Refresh /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>刷新登录用户Token</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>
                <div class="form">
                    <el-button
                        type="primary"
                        @click="onRefreshToken"
                        :loading="loading"
                        >刷新登录用户Token</el-button
                    >                    
                </div>
            </el-collapse-item>
            <el-collapse-item title="修改用户密码" name="changepwd" class="opblock-post">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Edit /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>修改用户密码</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>              
                <div class="form">
                    <el-form ref="changePwdFormRef" :rules="cpwdrules" :model="changePwdForm">
                        <el-form-item v-if="authconf.enable_organization" prop="organization">
                            <el-input
                                v-model.trim="changePwdForm.organization"
                                placeholder="输入所在的组织代码"
                            />
                        </el-form-item>                      
                        <el-form-item prop="username">
                            <el-input v-model.trim="changePwdForm.username" placeholder="用户帐号" />
                        </el-form-item>
                        <el-form-item prop="credential">
                            <el-input
                                type="password"
                                show-password
                                v-model.trim="changePwdForm.credential"
                                placeholder="原密码"
                            />
                        </el-form-item>
                        <el-form-item prop="new_credential">
                            <el-input
                                type="password"
                                show-password
                                v-model.trim="changePwdForm.new_credential"
                                placeholder="新密码"
                            />
                        </el-form-item>
                        <el-form-item v-if="authconf.enbale_captcha" prop="captcha_code">
                            <el-input
                                v-model.trim="changePwdForm.captcha_code"
                                placeholder="输入图片验证码"
                                style="width: calc(100% - 100px); padding-right: 20px;"
                            />                            
                            <img :src="changepwdCodeUrl" style="height: 40px;" @click="getChangePwdCode">
                        </el-form-item>
                        <el-form-item>
                            <el-button
                                type="primary"
                                @click="onChangePwd(changePwdFormRef)"
                                :loading="loading"
                                >修改密码</el-button
                            >
                        </el-form-item>
                    </el-form>
                </div>
            </el-collapse-item>
            <el-collapse-item v-if="authconf.enbale_captcha" title="获取图像验证码" name="code_image" class="opblock-post">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Picture /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>获取图像验证码</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>
                <div class="form">
                  <span class="text">使用GET 请求/api/auth/code_image，将获得随机的图像，同时，需要在成功返回后，将res.data.code_id，保存到后续的请求的表单数据中。</span>
                  <span class="text">而将res.data.image_url中保存的是Base64后的PNG图片信息。可以将 image.url = 'data:image/png;base64,' + res.data.image_url。</span>
                  <MdPreview editorId="code_image_id" :modelValue="codeimage_code" />
                  <img :src="demoCodeUrl" style="height: 40px;" @click="getDemoCode">
                  <el-button
                      type="primary"
                      @click="getDemoCode"
                      :loading="loading"
                      >获取Captcha图像</el-button
                  >
                </div>
            </el-collapse-item>
            <el-collapse-item title="退出退录状态" name="logout" class="opblock-put">
                <template #title>
                  <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><SwitchButton /></el-icon></span>
                  </div>
                  <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                      <a class="nostyle">
                        <span>退出退录状态</span>
                      </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                  </div>
                </template>              
                <div class="form">
                  <MdPreview editorId="logout_id" :modelValue="logout_code" />
                    <el-button
                        type="primary"
                        @click="onUserLogout"
                        :loading="loading"
                        >退出登录</el-button
                    >
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
  import { getTimeState } from "@/utils/utils";
  import { GlobalStore } from "@/stores";
  import type { FormInstance, FormRules } from "element-plus";
  import { get_javascript_axio_code_md, get_java_code_md, get_curl_code_md, get_javascript_packaged_code_md, get_rhai_code_md } from "@/utils/codetemplate";
  import { MdPreview, MdCatalog } from 'md-editor-v3';
  import 'md-editor-v3/lib/preview.css';

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
  const loginForm = reactive<any>({ username: "", credential: "", captcha_id: "", captcha_code: "" });
  const changePwdFormRef = ref();
  const changePwdForm = reactive<any>({ username: "", credential: "", new_credential: "", captcha_id: "", captcha_code: "" });  
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
  const demoCodeUrl = ref<string>();
  const loginCodeUrl = ref<string>();
  const changepwdCodeUrl = ref<string>();

  onMounted(() => {
    // 监听enter事件（调用登录）
    document.addEventListener("keydown", onKeyDown);
  });
  
  const onKeyDown = (e: any) => {
    if (e.code === "Enter" || e.code === "enter" || e.code === "NumpadEnter") {
      if (loading.value) return;
      onLogin(loginFormRef.value);
    }
  };



  async function call_api_docuemnt(path: string, inv: any, data: any) {
    javascript_code.value = get_javascript_axio_code_md(path, inv, data);
    java_code.value = get_java_code_md(path, inv, data);
    curl_code.value = get_curl_code_md(path, inv, data);
    packed_code.value = get_javascript_packaged_code_md(path, inv, data);
  }
  
  // 登录
  const onLogin = (formEl: FormInstance | undefined) => {
    if (!formEl) return;
    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
      if (!valid) return;
      loading.value = true;
      try {
        console.log('call api')
        call_api_docuemnt("/api/auth/login", { method: "POST" }, loginForm);
        const data = await call_api("/api/auth/login", "POST", loginForm);
        console.log(data)
        jsonbody.value = data
        if (data.status !== 200) {
          ElMessage.error(data.message);
          return;
        }
  
        globalStore.setApiToken(data.data.token);
      } finally {
        loading.value = false;
      }
    });
  };

  const onChangePwd = (formEl: FormInstance | undefined) => {
    if (!formEl) return;
    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
      if (!valid) return;
      loading.value = true;
      try {
        console.log('call api')
        call_api_docuemnt("/api/auth/change_pwd", { method: "POST" }, changePwdForm);
        const data = await call_api("/api/auth/change_pwd", "POST", changePwdForm);
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

  const onGetUserInfo = () => {
    call_api_docuemnt("/api/auth/info", { method: "GET" }, {});
    call_api("/api/auth/info", "GET", {}).then(res => {
        jsonbody.value = res
    }).catch(ex => {
        ElMessage.error(JSON.stringify(ex));
    })
  };

  const onRefreshToken = () => {
    call_api_docuemnt("/api/auth/refresh", { method: "GET" }, {});
    call_api("/api/auth/refresh", "GET", {}).then(res => {
        jsonbody.value = res
    }).catch(ex => {
        ElMessage.error(JSON.stringify(ex));
    })
  };

  const onExchange = (formEl: FormInstance | undefined) => {
    if (!formEl) return;
    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
      if (!valid) return;
      loading.value = true;
      try {
        console.log('call api')
        call_api_docuemnt("/api/auth/exchange", { method: "POST" }, loginForm);
        const data = await call_api("/api/auth/exchange", "POST", loginForm);
        console.log(data)
        jsonbody.value = data
        if (data.status !== 200) {
          ElMessage.error(data.message);
          return;
        }
  
        globalStore.setApiToken(data.data.token);
      } finally {
        loading.value = false;
      }
    });
  };

  const onUserLogout = () => {
    globalStore.setApiToken(null);
    ElMessage.success("退出成功。清除JWT Token。")
  };

  const getLoginCode = () => {
    call_api("/api/auth/code_image", "GET", {}).then(res => {
        loginCodeUrl.value = 'data:image/png;base64,' + res.data.image_url
        loginForm.captcha_id = res.data.code_id
    })
  }

  const getChangePwdCode = () => {
    
    call_api("/api/auth/code_image", "GET", {}).then(res => {
        changepwdCodeUrl.value = 'data:image/png;base64,' + res.data.image_url
        changePwdForm.captcha_id = res.data.code_id
    })
  }

  const getDemoCode = () => {
    call_api_docuemnt("/api/auth/code_image", { method: "GET" }, {});
    call_api("/api/auth/code_image", "GET", {}).then(res => {
        jsonbody.value = res
        demoCodeUrl.value = 'data:image/png;base64,' + res.data.image_url
    })
  }



  onActivated(() => {
      console.log("activate")
      getLoginCode()
      getChangePwdCode()
      getAuthorization();
  })

  onUpdated(() => {
      console.log("onUpdate")
      getLoginCode()
      getChangePwdCode()
      getAuthorization();
  })  
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  