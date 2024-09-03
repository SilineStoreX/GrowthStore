<template>
  <el-dialog
    v-model="props.visible"
    title="添加扩展服务"
    width="800"
    align-center
    @close="onDialogClosed"
  >
    <el-table :data="tables"  max-height="300" highlight-current-row  @selection-change="handleSelectionChange">
        <el-table-column label="协议名" prop="protocol" />
        <el-table-column label="插件模块" prop="plugin_dylib" width="200px" />
        <el-table-column label="类型" prop="plugin_type" />
        <el-table-column label="日志级别" prop="logger" />
        <el-table-column label="引用名" prop="name">
          <template #default="scoped">
            <el-input v-model="scoped.row.name" />
          </template>
        </el-table-column>
        <el-table-column type="selection" width="55" />
    </el-table>
    <template #footer>
      <div class="dialog-footer">
        <el-button @click="$emit('update:visible', false)">取消</el-button>
        <el-button type="primary" @click="onConfirm">
          确认
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script lang="ts" setup name="config">
import { plugin_list, update } from "@/http/modules/management";
import { useRoute } from "vue-router";
import { mergeProps, onMounted, ref, watch } from "vue";
const props = defineProps<{ visible: boolean }>();
const emit = defineEmits(['update:visible', 'datasync'])
const tables = ref<Array<any>>([])
const selections = ref<Array<any>>([])
const query = ref<any>({})
const route = useRoute()

function handlePluginList() {
  var ns = route.query.ns
  var q = query.value

  plugin_list().then(res => {
    tables.value = res.data.filter((p: any) => p.plugin_type !== 'lang')
  }).catch(ex => {
    console.log(ex)
  })
} 

function onDialogClosed() {
  emit('update:visible', false)
}

function handleSelectionChange(e: any) {
  console.log(e)
  selections.value = e
}

function onConfirm() {
    var ns = route.query.ns as string
    var tbls = selections.value.map(v => {
      return {
        name: v.name,
        protocol: v.protocol,
        enable: true,
        config: v.name + ".pltoml",
      }
    })

    update(ns, 'plugin', tbls).then(res => {
      emit('update:visible', false)
      emit('datasync', true)
    }).catch(ex => {
      console.log(ex)
    })
}

onMounted(() => {
    console.log("config");
    handlePluginList()
});
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
