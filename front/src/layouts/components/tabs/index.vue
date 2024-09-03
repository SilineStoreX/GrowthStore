<template>
  <el-tabs
    v-model="tabsMenuValue"
    type="card"
    class="demo-tabs"
    @tab-change="tabChange"
    @tab-remove="removeTab"
  >
    <el-tab-pane
      v-for="item in tabsMenuList"
      :key="item.path"
      :label="item.title"
      :name="item.path"
      :closable="item.close"
    >
      <template #label>
        <el-dropdown ref="dropdown1" trigger="contextmenu" @command="onCommandHandle">
          <span class="el-dropdown-link custom-tabs-label">
            <el-icon class="tabs-icon" v-show="item.icon">
              <component :is="item.icon"></component>
            </el-icon>
            <span>{{ item.title }}</span>
          </span>
          <template #dropdown>
            <el-dropdown-menu :data-item="item">
              <el-dropdown-item command="refresh">刷新</el-dropdown-item>
              <el-dropdown-item command="current" :disabled="!item.close">关闭当前</el-dropdown-item>
              <el-dropdown-item command="others">关闭其它</el-dropdown-item>
              <el-dropdown-item command="left">关闭左侧</el-dropdown-item>
              <el-dropdown-item command="right">关闭右侧</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </template>
    </el-tab-pane>
  </el-tabs>
</template>

<script lang="ts" setup>
import { ref, computed, watch, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { TabPaneName } from "element-plus";
import { GlobalStore } from "@/stores";
import { MenuStore } from "@/stores/modules/menu";

const route = useRoute();
const router = useRouter();
const globalStore = GlobalStore();
const menuStore = MenuStore();
const tabsMenuValue = ref(route.fullPath);
const tabsMenuList = computed(() => globalStore.tabsMenuList);

// 监听路由的变化
watch(
  () => route.fullPath,
  () => {
    // if (route.meta.isFull) return;
    tabsMenuValue.value = route.fullPath;
    const tabsParams = {
      icon: route.meta.icon as string,
      title: route.meta.title as string,
      path: route.fullPath,
      name: route.name as string,
      close: !route.meta.isAffix,
    };
    globalStore.addTabs(tabsParams);
    route.meta.isKeepAlive && menuStore.addKeepAliveName(route.name as string);
  },
  {
    immediate: true,
  }
);

onMounted(() => {
  initTabs();
});

// 初始化需要固定的标签
const initTabs = () => {
  menuStore.flatMenuList.forEach((item) => {
    if (item.meta.isAffix && !item.meta.isFull) {
      const tabsParams = {
        icon: item.meta.icon,
        title: item.meta.title,
        path: item.path,
        name: item.name,
        close: !item.meta.isAffix,
      };
      globalStore.addTabs(tabsParams);
    }
  });
};

const removeTab = (fullPath: string) => {
  const name =
    globalStore.tabsMenuList.filter((item) => item.path == fullPath)[0].name ||
    "";
  menuStore.removeKeepAliveName(name);
  globalStore.removeTabs(fullPath, fullPath == route.fullPath);
};

const removeTabOthers = (fullPath: string) => {
  globalStore.closeMultipleTab(fullPath);
};

const removeLeftTab = (fullPath: string) => {
  globalStore.closeLeftTab(fullPath);
};

const removeRightTab = (fullPath: string) => {
  globalStore.closeRightTab(fullPath);
};

const onCommandHandle = (cmd: string, dt: any) => {
  let r = dt && dt.parent && dt.parent.attrs['data-item'];
  if (cmd === "current") {
    if (r.path !== '/home') {
      removeTab(r.path);
    }
  } else if (cmd === "refresh") {
    window.location.reload()
  } else if (cmd === "others") {
    if (r.path !== '/home') {
      removeTabOthers(r.path)
    }
  } else if (cmd === "left") {
    removeLeftTab(r.path)
  } else if (cmd === "right") {
    removeRightTab(r.path)
  }
}

const tabChange = (path: TabPaneName) => {
  const fullPath = path as string;
  router.push(fullPath);
};
</script>

<style lang="scss" scoped>
.demo-tabs > .el-tabs__content {
  padding: 32px;
  color: #6b778c;
  font-size: 32px;
  font-weight: 600;
}
.demo-tabs .custom-tabs-label .el-icon {
  vertical-align: middle;
}
.demo-tabs .custom-tabs-label span {
  vertical-align: middle;
  margin-left: 4px;
}
</style>
