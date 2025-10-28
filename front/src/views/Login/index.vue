<template>
  <div class="login-container">
    <contact-us :visible="showContactUsDialog" title="注册联系人信息" :hook="currentHook" @update:visible="handleDialogVisibleChange"></contact-us>
    <license :visible="showLicenseDialog" title="许可协议（Apache）" src="/SERVICE-LICENSE-APACHE" :hook="currentHook" @update:visible="handleDialogVisibleChange"></license>
    <license :visible="showAggreeDialog" title="服务条款" src="/SERVICE-AGGREEMENT" :hook="currentHook" @update:visible="handleDialogVisibleChange"></license>
    <el-dialog
      v-model="showmore_dlg"
      :title="'关于' + version_info.display_name + ' v' + version_info.version"
      width="600"
      align-center
      :close-on-click-modal="false"
      :close-on-press-escape="false"      
      class="lic-dialog"
      @close="hideAboutMore">
      <div class="aboutcontent">
        <div class="about">{{ version_info.about }}</div>
        <div class="about"><span class="title">版本 </span> {{ version_info.version }}</div>
        <div class="about"><span class="title">CPU架构 </span> {{ version_info.arch }}，<span class="title">核心数量 </span> {{ version_info.cpu_cores }}</div>
        <div class="about"><span class="title">系统家族 </span> {{ version_info.family }}</div>
        <div class="about"><span class="title">操作系统 </span> {{ version_info.os_long_version }} 内核({{ version_info.kernel_long_version }})</div>
        <div class="about"><span class="title">作者 </span> {{ version_info.author }}</div>
      </div>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="primary" @click="hideAboutMore">确定</el-button>
        </div>
      </template>
    </el-dialog>    
    <div class="login-left">
      <div class="affix">
        <span class="app_title">
          
        </span>
      </div>
      <div class="form-box">
        <div class="form">
          <span>管理员登录</span>
          <el-form ref="loginFormRef" :rules="rules" :model="loginForm">
            <el-form-item prop="username">
              <el-input v-model.trim="loginForm.username" placeholder="帐号" />
            </el-form-item>
            <el-form-item prop="password">
              <el-input
                type="password"
                show-password
                v-model.trim="loginForm.password"
                placeholder="密码"
              />
            </el-form-item>
            <el-form-item>
              <el-switch v-model="isRemember" active-text="记住我" />
            </el-form-item>
            <el-form-item class="buttons">
              <el-button
                type="primary"
                @click="onLogin(loginFormRef)"
                :loading="loading"
                >登录</el-button
              >
              <el-button
                type="danger"
                @click="onSignup"
                >注册联系人</el-button
              >
            </el-form-item>
          </el-form>
        </div>
        <div class="bottaffix">
          <el-checkbox v-model="aggreed"/><span class="aggr"> 我同意<a href="#" class="aggreement" @click="onShowServiceAggr">服务条款</a>&<a href="#" class="app_license" @click="onShowLicense">许可协议(Apache)</a></span>
          <div class="version" @click="showAboutMore" style="cursor: pointer;">版本号：v{{ version_info.version }} {{ version_info.os_long_version && version_info.os_long_version !== '' ? '@' + version_info.os_long_version : ''}}</div>
        </div>
      </div>
    </div>
    <div class="login-right">
      
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from "vue";
import { Login } from "@/http/interface";
import { loginApi } from "@/http/modules/login";
import { ElMessage, ElNotification } from "element-plus";
import { ElMessageBox } from 'element-plus'
import { useRouter } from "vue-router";
import { getTimeState } from "@/utils/utils";
import { GlobalStore } from "@/stores";
import ContactUs from "./contact.vue"
import License from "./license.vue"
import { MenuStore } from "@/stores/modules/menu";
import { initDynamicRouter } from "@/routers/modules/dynamicRouter";
import type { FormInstance, FormRules } from "element-plus";
import { GITEE_URL } from "@/config/config";
import { fetch_auth_config, rsa_encrypt_text } from "@/components/chimescoresec/chimescoresec"
import { version_info_get } from "@/http/modules/performance";

const globalStore = GlobalStore();
const menuStore = MenuStore();
const router = useRouter();
// form表单规则校验
const loginFormRef = ref();
const loginForm = reactive<Login.ReqLoginForm>({ username: "", password: "" });
const rules = reactive<FormRules>({
  username: [{ required: true, message: "请输入帐号" }],
  password: [{ required: true, message: "请输入密码" }],
});
const loading = ref(false);
const isRemember = ref(false);
const currentHook = ref({});
const showContactUsDialog = ref(false);
const showLicenseDialog = ref(false);
const showAggreeDialog = ref(false);
const aggreed = ref(false);
const version_info = ref<any>({ version: "1.1.0" })
const showmore_dlg = ref<boolean>(false)

function showAboutMore() {
  showmore_dlg.value = true
}

function hideAboutMore() {
  showmore_dlg.value = false
}

onMounted(() => {
  // 监听enter事件（调用登录）
  fetch_auth_config().then(res => {
    console.log(res)
  })
  versionGet()
  document.addEventListener("keydown", onKeyDown);
});

const onKeyDown = (e: any) => {
  if (e.code === "Enter" || e.code === "enter" || e.code === "NumpadEnter") {
    if (loading.value) return;
    onLogin(loginFormRef.value);
  }
};

// 登录
const onLogin = (formEl: FormInstance | undefined) => {
  if (!aggreed.value) {
    ElMessageBox.alert("请您确认同意《服务条款》和《许可协议》？")
    return;
  }
  if (!formEl) return;
  formEl.validate(async (valid: boolean, _invalidFields?: any) => {
    if (!valid) return;
    loading.value = true;
    try {
      let sign_in = {
        username: loginForm.username,
        password: "rsa:" +  rsa_encrypt_text(loginForm.password), // rsa_encrypt(loginForm.password)
      }
      const data = await loginApi(sign_in);
      console.log(data)
      if (data.status !== 200) {
        ElMessage.error(data.message);
        return;
      }

      globalStore.setToken(data.data.token);
      // 添加动态路由
      await initDynamicRouter();

      // 清空 tabs、keepAlive 保留的数据
      globalStore.closeMultipleTab();
      menuStore.setKeepAliveName();

      router.push("home");
      ElNotification({
        title: getTimeState(),
        message: data.data.username,
        type: "success",
        duration: 3000,
      });
    } finally {
      loading.value = false;
    }
  });
};

const onSignup = () => {
  showContactUsDialog.value = true
}

const onShowLicense = () => {
  showLicenseDialog.value = true
}

const onShowServiceAggr = () => {
  showAggreeDialog.value = true
}

const handleDialogVisibleChange = (t: boolean) => {
  showContactUsDialog.value = t
  showLicenseDialog.value = t
  showAggreeDialog.value = t
  currentHook.value = {}
}

const onGitee = () => {
  window.open(GITEE_URL);
};

const versionGet = () => {
  version_info_get().then(res => {
    if (res.status === 200 || res.status === 0) {
      version_info.value = res.data
    }
  }).catch(ex => {
    console.log(ex)
  })
}
</script>

<style lang="scss" scoped>
@use "index.scss";
.version {
  padding: 5px;
  font-size: 12px;
}
.aboutcontent {
  padding: 20px;
}
.aboutcontent .about {
  line-height: 30px;
}
.aboutcontent .about .title {
  font-weight: 600;
}
</style>
