<template>
    <div class="home">
      <div>
        <el-form label-width="100px" :inline="false">
          <el-form-item label="MCP服务">
            <el-select v-model="cmdfrm.mcp_id" clearable style="width: 220px;">
              <el-option v-for="t in mcp_ids" :key="t" :value="t" :label="t">{{ t }}</el-option>
            </el-select>
            <el-button type="primary" @click="onQueryKeys" style="margin-left: 10px;">查询</el-button>
            <el-button type="danger" @click="refresh_tools" style="margin-left: 10px;">刷新MCP服务</el-button>
          </el-form-item>
          <el-form-item label="用户查询">
              <el-input v-model="cmdfrm.description" type="textarea" style="width: 620px; margin-right: 10px;"></el-input>
              <el-button type="success" @click="onQueryTest">召回测试</el-button>
          </el-form-item>
          <el-form-item label="召回参数" style="padding-top: -10px;">
              <el-form-item label="相似度大于"><el-input v-model="cmdfrm.score_threshold" /></el-form-item>
              <el-form-item label="Top-K"><el-input v-model="cmdfrm.top_k" /></el-form-item>
          </el-form-item>
        </el-form>
        <el-divider />
      </div>
      <el-table :data="rediskeys" v-loading="showLoading" ref="redistable" :expand-row-keys="expandedkeys" row-key="name" :height="scrollHeight - 190">
        <el-table-column type="expand">
              <template #default="scoped">
                  <JsonViewer :value="scoped.row.inputSchema" :expand-depth="5" copyable boxed sort></JsonViewer>
              </template>
        </el-table-column>
        <el-table-column prop="name" label="名称" width="560px" />
        <el-table-column prop="description" label="描述" show-overflow-tooltip />
        <el-table-column v-if="showSimilarity" prop="similarity" label="相似度" width="120px">
          <template #default="scoped">
            {{ scoped.row.similarity ? scoped.row.similarity.toFixed(5) : '' }}
          </template>  
        </el-table-column>
        <el-table-column label="操作" width="100px">
            <template #default="scoped">
              <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row)"/>
            </template>
        </el-table-column>
      </el-table>
      <div class="pager-container">
          <el-pagination :current-page="PageCurrent"
                         :page-size="PageSize" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="PageTotal" 
                         @size-change="handleSizeChange"
                         @current-change="handleCurrentChange"/>
      </div>
      <el-drawer title="工具调用测试" v-model="showEditForm" :with-header="true" :size="900">
        <el-form v-model="editfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="名称">
            <el-text class="mx-1" style="width: 700px">{{ editfrm.name }}</el-text>
          </el-form-item>
          <el-form-item label="详细描述">
            <el-text class="mx-1" style="width: 700px">{{ editfrm.description }}</el-text>
          </el-form-item>
          <el-form-item label="输入参数">
            <JsonViewer :value="editfrm.inputSchema" :expand-depth="5" copyable boxed sort style="width: 700px;"></JsonViewer>
          </el-form-item>
          <el-form-item label="测试参数输入">
            <el-input v-model="testfrm.input_params" type="textarea" :rows="4" style="width: 600px" placeholder="请根据输入参数描述的Schema来提供JSON格式的参数"></el-input>
            <el-button type="primary" @click="onTestInvoke" style="margin-left: 10px;">
              <el-icon class="el-icon--left"><CircleCheckFilled /></el-icon>
              执行
            </el-button>
          </el-form-item>
          <el-form-item v-if="testfrm.output" label="测试输出">
            <JsonViewer :value="testfrm.output" :expand-depth="5" copyable boxed sort style="width: 700px;"></JsonViewer>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="danger" @click="onDialogClosed">
            <el-icon class="el-icon--left"><CircleCloseFilled /></el-icon>
            关闭
          </el-button>
        </template>
      </el-drawer>
    </div>
  </template>
  
  <script lang="ts" setup name="prompts">
  import { onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import '@/utils/datetime.js'
  import JsonViewer from 'vue-json-viewer'
  import { toolcall_mcpids, toolcall_search, toolcall_invoke, toolcall_list, toolcall_refresh } from "@/http/modules/growthai";
  import { fetch_pluginnames } from "@/http/modules/management";
  import { ElMessage } from "element-plus";

  const route = useRoute()
  const mcp_ids = ref<Array<any>>([])
  const PageCurrent = ref<any>(1)
  const PageSize = ref<any>(20)
  const PageTotal = ref(0)
  const scrollHeight = ref<any>(600)
  const parent_prompts = ref<Array<any>>([])
  const rediskeys = ref<Array<any>>([])
  const tableData = ref<Array<any>>([])
  const default_rag_plugin = ref("")
  const cmdfrm = ref<any>({})
  const testfrm = ref<any>({})
  const editfrm = ref<any>({})
  const expandedkeys = ref<Array<any>>([])
  const showSimilarity = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)
  const showLoading = ref<boolean>(false)

  function handleSizeChange(e) {
    PageSize.value = e
    resizeData()
  }

  function handleCurrentChange(e) {
    PageCurrent.value = e
    resizeData()
  }

  function resizeData() {
    const start = (PageCurrent.value - 1) * PageSize.value;
    const end = start + PageSize.value;
    rediskeys.value = tableData.value.slice(start, end)
  }

  function onDialogClosed() {
    showEditForm.value = false
  }

  function onEditKey(row) {
    editfrm.value = row
    testfrm.value = {}
    showEditForm.value = true
  }

  function onTestInvoke() {
    var ns = route.query.ns as string
    let funcname = editfrm.value?.name
    let params = testfrm.value?.input_params
    if (funcname && funcname !== '') {
      let args = params
      try {
        args = JSON.parse(params)
      } catch(ex) {}

      var data = {
        name: funcname,
        arguments: args
      }
      toolcall_invoke(ns, default_rag_plugin.value, data).then((res: any) => {
        if (res.records) {
          var frm = testfrm.value
          if (typeof(res.records) === 'string') {
            try {
              frm.output = JSON.parse(res.records)
            }catch(ex) {
              frm.output = res.records
            }
          } else {
            frm.output = res.records
          }
          testfrm.value = frm
        }
      }).catch(ex => {
        ElMessage.warning("执行工具调用失败！" + ex)
      })
    }
  }

  function refresh_tools() {
    var ns = route.query.ns as string
    showLoading.value = true
    toolcall_refresh(ns, default_rag_plugin.value).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        mcp_ids.value = res.records
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }


  function fetch_mcp_ids() {
    var ns = route.query.ns as string
    showLoading.value = true
    toolcall_mcpids(ns, default_rag_plugin.value).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        mcp_ids.value = res.records
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function do_mcptools_list() {
    var ns = route.query.ns as string
    var queryfrm = cmdfrm.value
    var data = {
      mcp_id: queryfrm.mcp_id,
    }

    showLoading.value = true
    toolcall_search(ns, default_rag_plugin.value, data).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        PageTotal.value = res.records.length
        tableData.value = res.records
        PageCurrent.value = 1
        handleCurrentChange(1)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function do_mcptools_retrieval_test() {
    var ns = route.query.ns as string
    var queryfrm = cmdfrm.value
    var data = {
      document: queryfrm.description,
      top_n: queryfrm.top_k,
      score_threshold: queryfrm.score_threshold
    }

    showLoading.value = true
    toolcall_search(ns, default_rag_plugin.value, data).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        PageTotal.value = res.records.length
        tableData.value = res.records
        PageCurrent.value = 1
        handleCurrentChange(1)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function onQueryKeys() {
    showSimilarity.value = false
    do_mcptools_list()
  }

  function onQueryTest() {
    showSimilarity.value = true
    do_mcptools_retrieval_test()
  }

  function onResize() {
    scrollHeight.value = window.innerHeight - 240;
  }

  function fetchPluginNames() {
    var ns = route.query.ns as string
    fetch_pluginnames(ns, "rag").then(res => {
      let plugins = res.data
      if (plugins.length >= 1) {
        default_rag_plugin.value = plugins[0]
        fetch_mcp_ids()
      }
    }).catch(ex => {
      console.log(ex)
    })
  }

  onMounted(() => {
    fetchPluginNames()
    onResize()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  