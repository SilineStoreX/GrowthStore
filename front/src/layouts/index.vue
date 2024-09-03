<!-- 一次性加载 LayoutComponents -->
<template>
  <component :is="LayoutComponents[themeConfig.layout]" />
</template>

<script lang="ts" setup>
import { computed, type Component, onMounted } from "vue";
import { GlobalStore } from "@/stores";
import LayoutRow from "./LayoutRow/index.vue";
import LayoutColumn from "./LayoutColumn/index.vue";
import { getUserInfo } from "@/http/modules/user";

const LayoutComponents: { [key: string]: Component } = {
  row: LayoutRow,
  column: LayoutColumn,
};

const globalStore = GlobalStore();
const themeConfig = computed(() => globalStore.themeConfig);

onMounted(() => {
  getInfo();
});

const getInfo = async () => {
  const { data } = await getUserInfo();
  globalStore.setUserInfo(data);
};
</script>

<style lang="scss" scoped>
.layout {
  min-width: 760px;
  height: 100vh;
  :deep(.el-scrollbar__view) {
    height: 100%;
  }
}
</style>
