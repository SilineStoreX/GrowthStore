<template>
  <el-container class="layout row">
    <el-aside :width="isCollapse ? '65px' : '200px'">
      <el-scrollbar>
        <el-menu
          background-color="#333"
          text-color="#ddd"
          active-text-color="#fff"
          router
          :default-active="activePath"
          :collapse="isCollapse"
          :collapse-transition="false"
          unique-opened
        >
          <subMenu :menuList="menuList" />
        </el-menu>
      </el-scrollbar>
    </el-aside>
    <el-container>
      <el-header>
        <Header />
      </el-header>
      <Tabs />
      <el-main>
        <el-scrollbar>
          <Main />
        </el-scrollbar>
      </el-main>
      <el-footer>Footer</el-footer>
    </el-container>
  </el-container>
</template>

<script lang="ts" setup>
import Main from "../components/Main/index.vue";
import Header from "../components/Header/index.vue";
import subMenu from "../components/Menu/subMenu.vue";
import Tabs from "../components/tabs/index.vue";
import { ref, computed, watch } from "vue";
import { useRoute } from "vue-router";
import { GlobalStore } from "@/stores";
import { MenuStore } from "@/stores/modules/menu";

const globalStore = GlobalStore();
const menuStore = MenuStore();
const menuList = computed(() => menuStore.authMenuList);
const isCollapse = computed(() => globalStore.themeConfig.isCollapse);

// 获取当前路由路径
const route = useRoute();
const activePath = ref();

// 监听路由的变化（防止浏览器后退/前进activePath不变化 ）
watch(
  () => route.fullPath,
  () => {
    if (route.meta.isFull) return;
    activePath.value = route.fullPath;
  },
  {
    immediate: true,
  }
);
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>

<style lang="scss">
.row {
  .el-menu-item {
    &.is-active {
      background-color: var(--el-menu-hover-bg-color);

      &::before {
        content: "";
        position: absolute;
        top: 0;
        bottom: 0;
        left: 0;
        width: 4px;
        background: var(--el-color-primary);
      }
    }
  }

  .el-sub-menu__title:hover {
    background: transparent;
  }
}
</style>
