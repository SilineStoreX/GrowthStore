<template>
  <el-container class="layout column">
    <el-header>
      <span class="app_title"></span>
      <Header />
    </el-header>
    <el-container ref="computedContainer" class="column-inner">
      <el-dialog
        v-model="showAddNamesapceDialog"
        title="添加Namespace"
        width="500"
        align-center
        @close="onDialogClosed"
      >
        <el-form :model="conf" label-width="180px" :inline="true" style="max-width: 460px">
            <el-form-item label="文件名">
                <el-input v-model="conf.filename" style="width: 240px;"/>
            </el-form-item>
            <el-form-item label="Namespace">
                <el-input v-model="conf.namespace" style="width: 240px;"/>
            </el-form-item>
        </el-form>
        <template #footer>
          <div class="dialog-footer">
            <el-button @click="onDialogClosed">取消</el-button>
            <el-button type="primary" @click="onConfirm">确认</el-button>
          </div>
        </template>
      </el-dialog>      
      <PageSplit :distribute="0.15" :lineThickness="6" :isVertical="true" @resizeLineStartMove="onresizeLineStartMove" @resizeLineMove="onResizeLineMove" @resizeLineEndMove="onresizeLineEndMove">
        <template v-slot:first>
            <el-scrollbar>
              <el-input placeholder="请输入项目或Namespace进行检索">
                <template #append>
                  <el-button @click="onAddNamespace">新增</el-button>
                </template>
              </el-input>
              <el-tree-v2
                style="max-height: calc(100vh);"
                :data="projects"
                :highlight-current="true"
                :props="props"
                :item-size="32"
                :height="treeViewHeight"
                @current-change="nodeChanged"
                @node-expand="nodeExpanded"
              >
                <template #default="{node, data}">
                  <el-icon>
                    <component :is="data.icon" />
                  </el-icon>
                  <span>{{ data.label }}</span>
                </template>
              </el-tree-v2>
            </el-scrollbar>
        </template>
        <template v-slot:second>
          <el-container>
            <Tabs />
            <el-main>
                <Main />
            </el-main>
            <el-footer></el-footer>
          </el-container>
        </template>
      </PageSplit>
    </el-container>
  </el-container>
</template>

<script lang="ts" setup>
import Main from "../components/Main/index.vue";
import Header from "../components/Header/index.vue";
import subMenu from "../components/Menu/subMenu.vue";
import Tabs from "../components/tabs/index.vue";
import { fetchNamespaces, config_create } from "@/http/modules/management";
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { GlobalStore } from "@/stores";
import { MenuStore } from "@/stores/modules/menu";
import { useRouter } from "vue-router";
import PageSplit from "vue3-page-split";
import "vue3-page-split/dist/style.css";
import { ElNotification } from "element-plus";

interface Tree {
  id: string
  label: string
  children?: Tree[]
}
const router = useRouter();
const globalStore = GlobalStore();
const menuStore = MenuStore();
const menuList = computed(() => menuStore.authMenuList);
const isCollapse = computed(() => globalStore.themeConfig.isCollapse);
const showAddNamesapceDialog = ref<boolean>(false)
// 获取当前路由路径
const route = useRoute();
const activePath = ref();
const conf = ref<any>({})
const computedContainer = ref(null);
const treeViewHeight = ref(1024);
let observer = null;
  
const handleResize = () => {
  if (computedContainer.value && computedContainer.value.$el) {
    const width = computedContainer.value.$el.offsetWidth;
    const height = computedContainer.value.$el.offsetHeight;
    console.log(`Size: width=${width}, height=${height}`);
    treeViewHeight.value = height - 40;
  }
};

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

watch(
  () => globalStore.namespaceTree,
  () => {
    fetchNamespacesTree();
  },
  {
    immediate: true,
  }
);

const props = {
  value: 'id',
  label: 'label',
  icon: 'icon',
  children: 'children',
}

const projects = ref<Tree[]>([])

const user_login_menu_item = ref<any>({
  id: 'global:login',
  label: '登录与认证测试',
  icon: 'Avatar',
  children: []
})

const user_conf_menu_item = ref<any>({
  id: 'global:authorization',
  label: '登录与认证配置',
  icon: 'Avatar',
  children: [user_login_menu_item.value]
})



function onresizeLineStartMove() {
  console.log("onresizeLineStartMove");
}

function onResizeLineMove(e: any) {
  console.log("onResizeLineMove :>> ", e);
}

function onresizeLineEndMove() {
  console.log("onresizeLineEndMove");
}

function nodeChanged(node: any) {
  console.log('nodeChanged', node)
  if (node.id === 'global:authorization') {
    router.push(`/storex/authorization?id=${node.id}`)
  } else if (node.id === 'global:login') {
    router.push(`/storex/login?id=${node.id}`)
  } else if (node.id.indexOf(':') >= 0) {
    var suff = node.id.substring(node.id.indexOf(":") + 1)
    console.log(node.id, suff)
    if (suff === '_redis') {
      router.push("/storex/redis?ns=" + node.id.substring(0, node.id.indexOf(":")))
    } else if (suff === '_es') {
      router.push("/storex/elasticsearch?ns=" + node.id.substring(0, node.id.indexOf(":")))
    } else if (suff === '_config') {
      router.push("/storex/namespaces?ns=" + node.id.substring(0, node.id.indexOf(":")))
    } else {
      router.push("/storex/invoker?ns=" + node.id)
    }
  } else {
    router.push("/storex/namespaces?ns=" + node.id)
  }

}

function nodeExpanded(node: any) {
  console.log('nodeExpanded', node)
  
}

function fetchNamespacesTree() {
  fetchNamespaces().then(res => {
    projects.value = [user_conf_menu_item.value, ...res.data]
  }).catch(ex => {})
}

function onAddNamespace() {
  showAddNamesapceDialog.value = true
}

function onDialogClosed() {
  showAddNamesapceDialog.value = false
}

function onConfirm() {
  var ns = conf.value.namespace as string
  console.log('Config: ' + JSON.stringify(conf.value))
  config_create('config', ns, conf.value).then(res => {
      console.log('Save Success')
      fetchNamespacesTree()
      onDialogClosed()
      if (res.status === 200 || res.status === 0) {
        ElNotification({
          title: '提示',
          message: '创建Namespace成功。',
          type: "success",
          duration: 3000,
        })
      } else {
        ElNotification({
          title: '提示',
          message: '创建Namespace失败。' + res.message,
          type: "error",
          duration: 3000,
        })
      }
  }).catch((ex: any) => {
      console.log("exception ", ex)
      ElNotification({
        title: '提示',
        message: '创建Namespace失败。',
        type: "error",
        duration: 3000,
      })
  })
}

fetchNamespacesTree()

onMounted(() => {
  handleResize(); // 初始大小
  observer = new ResizeObserver(handleResize);
  if (computedContainer.value && computedContainer.value.$el) {
    observer.observe(computedContainer.value.$el);
  }
});
 
onUnmounted(() => {
  if (observer) {
    if (computedContainer.value && computedContainer.value.$el) {
      observer.unobserve(computedContainer.value.$el);
    }
  }
});

</script>

<style lang="scss" scoped>
@import "index.scss";
</style>

<style lang="scss">
.column {
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

