<template>
  <router-view v-slot="{ Component, route }">
      <keep-alive :include="menuStore.keepAliveName">
        <component :is="Component" :key="route.fullPath" />
      </keep-alive>
  </router-view>
</template>

<script lang="ts" setup>
import { MenuStore } from "@/stores/modules/menu";
const menuStore = MenuStore();
</script>
