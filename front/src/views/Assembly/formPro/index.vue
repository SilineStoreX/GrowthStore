<template>
  <div class="form">
    <el-input v-model="value"></el-input>
  </div>
</template>

<script lang="ts" setup name="form-pro">
import { ref, onMounted } from "vue";
import { MenuStore } from "@/stores/modules/menu";
const menuStore = MenuStore();
const value = ref("");

onMounted(() => {});
</script>
