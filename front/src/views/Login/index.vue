<template>
  <div class="login-container">
    <contact-us :visible="showContactUsDialog" title="注册联系人信息" :hook="currentHook" @update:visible="handleDialogVisibleChange"></contact-us>
    <license :visible="showLicenseDialog" title="许可协议（Apache）" src="/SERVICE-LICENSE-APACHE" :hook="currentHook" @update:visible="handleDialogVisibleChange"></license>
    <license :visible="showAggreeDialog" title="服务条款" src="/SERVICE-AGGREEMENT" :hook="currentHook" @update:visible="handleDialogVisibleChange"></license>
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
import { rsa_encrypt } from "@/utils/encryption";
import { GlobalStore } from "@/stores";
import ContactUs from "./contact.vue"
import License from "./license.vue"
import { MenuStore } from "@/stores/modules/menu";
import { initDynamicRouter } from "@/routers/modules/dynamicRouter";
import type { FormInstance, FormRules } from "element-plus";
import { GITEE_URL } from "@/config/config";

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
        password: "rsa:" + rsa_encrypt(loginForm.password)
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
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
