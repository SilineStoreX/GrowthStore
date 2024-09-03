<template>
    <el-dialog
      v-model="showEditForm"
      title="添加/编辑索引"
      width="800"
      align-center
      @close="onDialogClosed"
    >
      <el-form :model="hook" label-width="100px" :inline="true" style="max-width: 600px">
            <el-form-item label="索引名称">
                <el-input v-model="hook.index" rows="10" style="width: 600px" />
            </el-form-item>
            <el-form-item v-if="updateIndexMode" label="更新索引内容">
              <el-radio-group v-model="hook.mode" style="width: 600px">
                    <el-radio-button value="aliases">Aliases</el-radio-button>
                    <el-radio-button value="mappings">Mappings</el-radio-button>
                    <el-radio-button value="settings">Settings</el-radio-button>
                </el-radio-group>
            </el-form-item>
            <el-form-item label="更新内容(JSON)">
                <el-input type="textarea" v-model="hook.jsonbody" rows="10" style="width: 600px" />
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
    <div class="home">
      <div>
        <el-form label-width="100px" :inline="true">
          <el-form-item label="扩展名称">
              <el-select v-model="select_plugin" style="width: 200px" @change="onQueryKeys">
                <el-option v-for="it in pluginnames" :value="it">{{ it }}</el-option>
              </el-select>                
          </el-form-item>
          <el-form-item label="索引">
              <el-input v-model="cmdfrm.key_prefix"></el-input>
          </el-form-item>
          <el-form-item>
              <el-button @click="onQueryKeys">查询</el-button>
              <el-button @click="onCreateIndex">创建</el-button>
          </el-form-item>
        </el-form>
      </div>
      <el-table :data="rediskeys" ref="redistable" :expand-row-keys="expandedkeys" row-key="uuid" @expand-change="onTableRowExpanded">
          <el-table-column type="expand">
              <template #default="scoped">
                  <JsonViewer :value="scoped.row.body" :expand-depth="5" copyable boxed sort></JsonViewer>
              </template>
          </el-table-column>          
          <el-table-column prop="index" label="索引" />
          <el-table-column prop="uuid" label="UUID" width="240px" />
          <el-table-column prop="health" label="健康" />
          <el-table-column prop="status" label="状态" />
          <el-table-column prop="pri" label="主片数" />
          <el-table-column prop="rep" label="复制数" />
          <el-table-column prop="docs.count" label="文档数" />
          <el-table-column prop="docs.deleted" label="删除文档" />
          <el-table-column prop="store.size" label="存储空间" />
          <el-table-column prop="pri.store.size" label="主片空间" />
          <el-table-column prop="dataset.size" label="数据集空间" />
          <el-table-column label="操作" width="100px">
              <template #default="scoped">
                <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row.index)"/>
                <el-popconfirm title="确认要删除吗?" @confirm="onDelKey(scoped.row.index)">
                    <template #reference>
                        <el-button type="danger" icon="Delete" circle />
                    </template>
                </el-popconfirm>
              </template>
          </el-table-column>
        </el-table>
    </div>
  </template>
  
  <script lang="ts" setup name="redis">
  import { onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import JsonViewer from 'vue-json-viewer'
  import { common_manage, fetch_pluginnames } from "@/http/modules/management";
  import { ElMessage } from "element-plus";

  const route = useRoute()
  const rediskeys = ref<Array<any>>([])
  const pluginnames = ref<Array<any>>([])
  const select_plugin = ref<string>("")
  const cmdfrm = ref<any>({})
  const expandedkeys = ref<Array<any>>([])
  const hook = ref<any>({})
  const updateIndexMode = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)

  function onDelKeys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    onDelKey(key)
  }

  function fetchPluginNames() {
    var ns = route.query.ns as string    
    fetch_pluginnames(ns, "elasticsearch").then(res => {
      pluginnames.value = res.data
      if (res.data.length >= 1) {
        select_plugin.value = res.data[0]
      }
      fetch_indexes()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function get_plugin_name() {
    let name = select_plugin.value
    if (name && name !== '') {
      return name
    } else {
      return "---"
    }
  }

  function onCreateIndex() {
    updateIndexMode.value = false
    let jsonbody = {
      aliases: {},
      mappings: {},
      settings: {}
    }
    hook.value = {jsonbody: JSON.stringify(jsonbody)}
    showEditForm.value = true
  }

  function onDialogClosed() {
    updateIndexMode.value = false
    showEditForm.value = false
  }

  function onEditKey(row) {
    updateIndexMode.value = true
    hook.value = {
      index: row,
      mode: "settings",
      jsonbody: "{}"
    }
    showEditForm.value = true    
  }


  function onConfirm() {
    var es = hook.value
    var ns = route.query.ns as string
    var method = updateIndexMode.value ? "update_" + es.mode + ":" + es.index: "create:" + es.index
    var jsonbody = {}
    try {
      jsonbody = JSON.parse(es.jsonbody)
    }catch(e) {}
    
    common_manage("elasticsearch", ns, get_plugin_name(), method, jsonbody).then(res => {
      if (res.status === 200 || res.status === 0) {
        fetch_indexes()
        showEditForm.value = false
      } else {
        ElMessage.warning("执行失败：" + res.message)
      }
    }).catch(ex => {
      console.log(ex)
      ElMessage.warning("执行失败：" + ex.message)
    })
  }

  function onDelKey(key) {
    var ns = route.query.ns as string
    var method = "delete:" + key
    common_manage("elasticsearch", ns, get_plugin_name(), method, {}).then(res => {
      fetch_indexes()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetch_indexes() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    var method = "list" + (key === '' ? '' : ':' + key)
    common_manage("elasticsearch", ns, get_plugin_name(), method, {}).then(res => {
      rediskeys.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onQueryKeys() {
    fetch_indexes()
  }

  function onTableRowExpanded(row: any, expanded: any[]) {
    console.log('ex', expanded)
    if (expanded.length > 0) {
      var ns = route.query.ns as string
      var method = "setting:" + row.index
      common_manage("elasticsearch", ns, get_plugin_name(), method, {}).then(res => {
          row.body = res.data
          expandedkeys.value = [row.uuid]
      });
    }
  }

  onMounted(() => {
    console.log("elasticsearch");
    fetchPluginNames()
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  