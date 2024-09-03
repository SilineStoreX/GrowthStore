<template>
  <div class="header">
    <el-dialog
        v-model="showChangePasswordDialog"
        :title="'修改' + userInfo.username + '的密码'"
        width="500"
        align-center
        :close-on-click-modal="false"
        @close="onDialogClosed"
      >
        <el-form :model="chpwd" label-width="140px" :inline="true" style="max-width: 460px">
            <el-form-item label="原密码">
                <el-input type="password" show-password v-model.trim="chpwd.password" style="width: 240px;"/>
            </el-form-item>
            <el-form-item label="新密码">
                <el-input type="password" show-password v-model.trim="chpwd.new_password" style="width: 240px;"/>
            </el-form-item>
            <el-form-item label="再输一次">
                <el-input type="password" show-password v-model.trim="chpwd.try_password" style="width: 240px;"/>
            </el-form-item>
        </el-form>
        <template #footer>
          <div class="dialog-footer">
            <el-button @click="onDialogClosed">取消</el-button>
            <el-button type="primary" @click="onConfirm">
              确认
            </el-button>
          </div>
        </template>
      </el-dialog>    
    <div class="header-left">
      <el-breadcrumb class="breadcrumb" separator-icon="ArrowRight">
        <transition-group name="breadcrumb">
          <el-breadcrumb-item
            v-for="item in breadcrumbList"
            :key="item.path"
          >
            <div class="inner-item">
              <el-icon>
                <component :is="item.meta.icon"></component>
              </el-icon>
              <span>{{ item.meta.title }}</span>
            </div>
          </el-breadcrumb-item>
        </transition-group>
      </el-breadcrumb>
    </div>
    <div class="header-right">
      <el-dropdown trigger="click" @command="handleI18n">
        <i class="iconfont icon-zhongyingwen"></i>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              :disabled="language && language === 'zh'"
              command="zh"
              >简体中文</el-dropdown-item
            >
            <el-dropdown-item :disabled="language === 'en'" command="en"
              >English</el-dropdown-item
            >
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <i class="iconfont icon-sousuo"></i>
      <i class="iconfont icon-lingdang"></i>
      <i
        :class="[
          'iconfont',
          isFullscreen ? 'icon-cancel-full-screen' : 'icon-full-screen',
        ]"
        @click="toggle"
      ></i>
      <span>{{ userInfo.username }}</span>
      <el-dropdown trigger="click">
        <el-avatar
          class="avatar"
          icon="UserFilled"
          fit="cover"
          :size="40"
          :src="userInfo.avatar"
        />
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item @click="changepwd">
              <el-icon>
                <User />
              </el-icon>
              <span>{{ $t("header.changepwd") }}</span>
            </el-dropdown-item>
            <el-dropdown-item @click="loginOut">
              <el-icon>
                <SwitchButton></SwitchButton>
              </el-icon>
              <span>{{ $t("header.logout") }}</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useFullscreen, useDebounceFn } from "@vueuse/core";
import { getBrowserLang } from "@/utils/utils";
import { modifyPassword } from "@/http/modules/login";
import { useI18n } from "vue-i18n";
import { GlobalStore } from "@/stores";
import { MenuStore } from "@/stores/modules/menu";
import { LayoutType } from "@/stores/interface";
import { useRoute, useRouter } from "vue-router";
import { LOGIN_URL } from "@/config/config";
import { nextTick } from "process";

const { toggle, isFullscreen } = useFullscreen();
const i18n = useI18n();
const drawer = ref(false);
const showChangePasswordDialog = ref(false);
const chpwd = ref<any>({});

const route = useRoute();
const router = useRouter();
const globalStore = GlobalStore();
const menuStore = MenuStore();
const language = computed((): string => globalStore.language);
const themeConfig = computed(() => globalStore.themeConfig);
const userInfo = computed(() => globalStore.userInfo);
const breadcrumbList = computed(
  () =>
    menuStore.breadcrumbList[route.matched[route.matched.length - 1].path] ?? []
);
const screenWidth = ref(0);

onMounted(() => {
  handleI18n(language.value || getBrowserLang());
  window.addEventListener("resize", lintenWindow, false);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", lintenWindow);
});

// 监听视图大小，折叠菜单
const lintenWindow = useDebounceFn(() => {
  screenWidth.value = document.body.clientWidth;
  if (!themeConfig.value.isCollapse && screenWidth.value < 1300)
    globalStore.setThemeConfig({ ...themeConfig.value, isCollapse: true });
  if (themeConfig.value.isCollapse && screenWidth.value > 1300)
    globalStore.setThemeConfig({ ...themeConfig.value, isCollapse: false });
});

// 折叠菜单
const collapse = () => {
  globalStore.setThemeConfig({
    ...themeConfig.value,
    isCollapse: !themeConfig.value.isCollapse,
  });
};

// 切换语言
const handleI18n = (lang: string) => {
  i18n.locale.value = lang;
  globalStore.updateLanguage(lang);
};

// 主题设置
const changeLayout = (val: LayoutType) => {
  globalStore.setThemeConfig({ ...themeConfig.value, layout: val });
};

// 退出登录
const loginOut = () => {
  ElMessageBox.confirm("是否确认退出登录", "提示", {
    confirmButtonText: "确定",
    cancelButtonText: "取消",
    type: "warning",
  })
    .then(() => {
      globalStore.setToken("");
      // router.push({ path: LOGIN_URL, replace: true });
      router.push({ name: 'login' })
    })
    .catch(() => {});
};

const onDialogClosed = () => {
  chpwd.value = {}
  showChangePasswordDialog.value = false
}

const changepwd = () => {
  showChangePasswordDialog.value = true
}

const onConfirm = () => {
  var cpwd = chpwd.value
  if (!cpwd.password || cpwd.password === '') {
    ElMessage.warning({message: "请输入的原密码!"})
    return;
  }
  if (!cpwd.new_password || cpwd.new_password === '') {
    ElMessage.warning({message: "请输入的新密码!"})
    return;
  }
  if (cpwd.new_password !== cpwd.try_password) {
    ElMessage.warning({message: "两次输入的新密码不匹配!"})
    return;
  }
  if (cpwd.new_password == cpwd.password) {
    ElMessage.warning({message: "输入的新密码与原密码相同，请重新输入!"})
    return;
  }  

  modifyPassword({ username: userInfo.value.username, password: cpwd.password, new_password: cpwd.new_password }).then(res => {
    if (res.status === 0 || res.status === 200) {
      ElMessage.success({message: "密码修改成功! 请重新登录。"})
      onDialogClosed()
      globalStore.setToken("");
      router.push({ name: 'login' })
    } else {
      ElMessage.warning({message: "密码修改失败，" + res.message })
    }
  }).catch(ex => {
    ElMessage.warning({message: "密码修改失败，" + ex.message })    
  })
}

</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
