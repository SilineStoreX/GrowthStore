<template>
    <div class="container">
        <add-hook :visible="showHookDialog" :hook="currentHook" @update:visible="handleHookDialogVisibleChange" @update:hook="handleUpdateHook" />
        <el-scrollbar>
            <el-form label-width="160px" :inline="false">
                <el-form-item label="插件协议">
                    <el-input v-model="data.protocol" disabled />
                </el-form-item>
                <el-form-item label="引用名称">
                    <el-input v-model="data.name" disabled />
                </el-form-item>                
                <el-form-item label="配置文件">
                    <el-input v-model="data.config" disabled />
                </el-form-item>
                <el-form-item label="启用">
                    <el-switch v-model="data.enable" />
                </el-form-item>
                <el-collapse v-model="activeNames">
                  <el-collapse-item title="RAG配置" name="info">                
                    <el-form-item label="文档识别接口">
                        <el-input v-model="rest_conf.doc_recognize_endpoint" />
                    </el-form-item>
                    <el-form-item label="Embedding处理端口">
                        <el-input v-model="rest_conf.embedding_endpoint" />
                    </el-form-item>
                    <el-form-item label="提示词保存接口">
                        <el-input v-model="rest_conf.prompt_save_uri" />
                    </el-form-item>
                    <el-form-item label="提示词召回接口">
                        <el-input v-model="rest_conf.prompt_retrieve_uri" />
                    </el-form-item>
                    <el-form-item label="Chunk保存接口">
                        <el-input v-model="rest_conf.chunk_save_uri" />
                    </el-form-item>
                    <el-form-item label="Chunk召回接口">
                        <el-input v-model="rest_conf.chunk_retrieve_uri" />
                    </el-form-item>
                    <el-form-item label="Chat BaseURL">
                        <el-input v-model="rest_conf.chat_base_uri" />
                        <el-text>此处填写由RAgent插件公开出来的URI路径</el-text>
                    </el-form-item>
                    <el-form-item label="Voice(ASR) BaseURL">
                        <el-input v-model="rest_conf.voice_base_uri" />
                        <el-text>此处填写由voice插件公开出来的URI路径</el-text>
                    </el-form-item>                    
                    <el-form-item prop="interval" label="间隔时间">
                      <el-input v-model="rest_conf.interval_second" placeholder="每次执行间隔指定秒数，如该值有效，则忽略CRON表达式" style="width: 270px" />
                      <el-form-item prop="cron_express" label="CRON表达式">
                        <el-input v-model="rest_conf.cron_express" placeholder="定时器任务的CRON表达式" style="width: 270px">
                          <template #append>
                            <el-button @click="onShowCronExpress">CRON表达式</el-button>
                          </template>
                        </el-input>
                        <el-dialog v-model="showCron">
                          <vue3-cron-plus
                            @change="changeCron"
                            @close="closeDialog"
                            max-height="600px"
                            i18n="cn">
                          </vue3-cron-plus>
                        </el-dialog>                        
                      </el-form-item>
                    </el-form-item>
                    <el-form-item label="转换任务方式">
                        <el-switch v-model="rest_conf.execute_each_task" />
                        <el-text>启用时，每个结构化文档任务运行在自己独立的Job中；关闭时，所有在转换任务在同一个Job线程中。</el-text>
                    </el-form-item>
                    <el-form-item prop="interval" label="转换任务间隔">
                      <el-input v-model="rest_conf.convert_interval_second" placeholder="每次执行间隔指定秒数，如该值有效，则忽略CRON表达式" style="width: 270px" />
                      <el-form-item prop="cron_express" label="CRON表达式">
                        <el-input v-model="rest_conf.convert_cron_express" placeholder="定时器任务的CRON表达式" style="width: 270px">
                          <template #append>
                            <el-button @click="onShowCronExpress">CRON表达式</el-button>
                          </template>
                        </el-input>
                        <el-dialog v-model="showCron">
                          <vue3-cron-plus
                            @change="changeCron"
                            @close="closeDialog"
                            max-height="600px"
                            i18n="cn">
                          </vue3-cron-plus>
                        </el-dialog>                        
                      </el-form-item>
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider />
            <template v-if="editingOrAdding !== 0 && rest_conf.docset_type !== 'provider'">
                <el-form ref="composeEditFormRef" :model="composeService" label-width="140">
                    <el-form-item label="ID">
                        <el-input v-model="composeService.mcp_id" />
                    </el-form-item>
                    <el-form-item label="MCP服务端口" prop="mcp_endpoint">
                        <el-input v-model="composeService.mcp_endpoint"></el-input>
                    </el-form-item>
                    <el-form-item label="AppKey"  prop="authorization">
                        <el-input v-model="composeService.authorization" />
                    </el-form-item>
                    <el-form-item label="SSE方式"  prop="sse">
                        <el-switch v-model="composeService.sse" />
                        <span style="color: #aaa;">只支持SSE方式或Streamable HTTP方式的MCP服务</span>
                    </el-form-item>
                    <el-form-item label="备注" prop="remark">
                        <el-input v-model="composeService.remark" type="textarea" placeholder="关于该服务的备注说明"  :rows="2"></el-input>
                    </el-form-item>
                </el-form>
                <div style="margin-top: 20px">
                    <el-button type="primary" @click="onSaveEditComposeService">保存服务</el-button>
                    <el-button @click="onCancelReturn">返回</el-button>
                </div>
            </template>
            <template v-else>
                <el-table v-if="rest_conf.docset_type !== 'provider'" :data="config_data">
                    <el-table-column prop="mcp_id" label="ID" width="80px" :show-overflow-tooltip="true" />
                    <el-table-column prop="mcp_endpoint" label="MCP服务端口" :show-overflow-tooltip="true" width="180px" />
                    <el-table-column prop="authorization" label="App Key" witdh="100px" :show-overflow-tooltip="true"/>
                    <el-table-column prop="sse" label="类型" :show-overflow-tooltip="true">
                      <template #default="scoped">
                        {{ scoped.row.sse ? 'SSE': 'Streamable HTTP' }}
                      </template>
                    </el-table-column>
                    <el-table-column prop="remark" label="备注" :show-overflow-tooltip="true" width="180px"/>
                    <el-table-column label="操作" width="100px">
                        <template #default="scoped">
                            <el-button type="primary" icon="Edit" circle @click="onEditComposeService(scoped.row)" />
                            <el-popconfirm title="确认要删除吗?" @confirm="onDeleteComposeService(scoped.row)">
                                <template #reference>
                                    <el-button type="danger" icon="Delete" circle />
                                </template>
                            </el-popconfirm>
                        </template>
                    </el-table-column>
                </el-table>
                <div style="margin-top: 20px">
                    <el-button @click="onAddComposeService">添加</el-button>
                </div>
            </template>
        </el-scrollbar>
      <div v-if="editingOrAdding === 0" class="drawer-footer">
        <el-button @click="$emit('update:visible', false)">关闭</el-button>
        <el-button type="primary" @click="onConfirm">
          保存
        </el-button>
        <el-popconfirm
            confirmButtonText="确定"
            cancelButtonText="取消"
            icon="el-icon-info"
            iconColor="red"
            width="280px"
            title="你确认要删除该查询服务的定义吗？"
            @confirm="handleRemove"
        >
            <template #reference>
            <el-button type="danger">删除</el-button>
            </template>
        </el-popconfirm>
      </div>
    </div>
</template>
  
  <script lang="ts" setup name="config">
  import { update, remove, metadata_get, config_get, config_save, lang_list, authorize_roles_get } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import { mergeProps, onMounted, ref, watch } from "vue";
  import { vue3CronPlus } from 'vue3-cron-plus'
  import { ElMessageBox, FormInstance } from "element-plus";
  import AddHook from "./add_hook.vue"

  const props = defineProps<{ data: any }>();
  const emit = defineEmits(['update:data', 'update:visible'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const activeName = ref<any>("query")
  const protocol_forms = ref<Array<any>>([])
  const config_data = ref<Array<any>>([])
  const rest_conf = ref<any>({})
  const editingOrAdding = ref<any>(0)
  const composeService = ref<any>({})
  const ScriptLangs = ref<Array<any>>([])
  const composeEditFormRef = ref<FormInstance>()
  const showHookDialog = ref<boolean>(false)
  const currentHook = ref<any>()
  const auth_roles = ref<Array<any>>([])
  const activeNames = ref<Array<any>>([])
  
  const showCron = ref<boolean>(false)
  
  const onShowCronExpress = () => {
    showCron.value = true
  }

  const closeDialog = () => {
    showCron.value = false
  }

  const changeCron = (val: any) => {
    if (typeof(val) === "string") {
      let cs = composeService.value
      cs.cron_express = val
      composeService.value = cs
    }
  }
  
  watch(
    () => [props.data.protocol, props.data.name],
    (newVal, oldVal) => {
      console.log('Watch for props ', newVal, oldVal)
      var ns = route.query.ns as string
      fetchMetadata(newVal[0])
      fetchConfig(newVal[0], ns, newVal[1])
    }
  )

  function fetchLang(){
    lang_list().then(res => {
      if (res.status === 0 || res.status === 200) {
        ScriptLangs.value = res.data
      }
    })
  }

  function fetchAuthRoles() {
    var ns = route.query.ns as string  
    authorize_roles_get().then(res => {
      auth_roles.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  } 
  
  function handleUpdate() {
    var ns = route.query.ns as string
    update(ns, 'plugin', [props.data]).then(_res => {
      let restconf = rest_conf.value
      config_save(props.data.protocol, ns, props.data.name, { ...restconf, services: config_data.value }).then(res => {
        if (res.status === 0 || res.status === 200) {
          emit("update:visible", false)
          emit("update:data", true)
        } else {
          ElMessageBox.alert('保存失败', "提示", { type: 'warning' })
        }
      }).catch(me => {
        ElMessageBox.alert('保存失败，' + me.description, "提示", { type: 'warning' })
      })

    }).catch(ex => {
      ElMessageBox.alert('保存插件信息失败, ' + ex.description, "提示", { type: 'warning' })
    })
  }


  function handleHookDialogVisibleChange(e: any) {
    showHookDialog.value = e
  }

  function fetchMetadata(schema: string) {
    metadata_get(schema).then(res => {
      console.log('metadata', res)
      protocol_forms.value = res as unknown as any[]
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetchConfig(schema: string, ns: string, name: string) {
    config_get(schema, ns, name).then(res => {
      rest_conf.value = res.data || {}
      config_data.value = res.data && res.data.services ? res.data.services : []
    }).catch(ex => {
      console.log(ex)
    })
  }

  function saveConfig(schema: string, ns: string, name: string) {
    config_save(schema, ns, name, config_data.value).then(res => {
      rest_conf.value = res.data || {}
      config_data.value = res.data && res.data.services ? res.data.services : []
    }).catch(ex => {
      console.log(ex)
    })
  }


  function onAddComposeService() {
    editingOrAdding.value = 1
    composeService.value = {}
  }

  function onCancelReturn() {
    composeEditFormRef.value?.resetFields()
    editingOrAdding.value = 0
  }

  // add the editing form to the services list
  function onSaveEditComposeService() {
    if (editingOrAdding.value === 1) {
        var composes = config_data.value
        composes.push(composeService.value)
        config_data.value = composes
    }
    editingOrAdding.value = 0
  }

  // del the spec Compose Service
  function onDeleteComposeService(raw) {
    var composes = config_data.value
    let index = composes.indexOf(raw) // 找到要删除的元素的索引，此处为 2
    composes.splice(index, 1)
    config_data.value = composes
  }

  function onEditComposeService(raw) {
    editingOrAdding.value = 2
    composeService.value = raw 
  }

  function handleRemove() {
    var ns = route.query.ns as string  
    remove(ns, 'plugin', [props.data.name]).then(_res => {
      emit("update:visible", false)
      emit("update:data", true)
    }).catch(ex => {
      console.log(ex)
    })
  }
  
  function handleSelectionChange(e: any) {
    console.log(e)
    selections.value = e
  }
  
  function onConfirm() {
    console.log("config: ", config_data.value)
    handleUpdate()
  }

  function onAddHook() {
    showHookDialog.value = true
    currentHook.value = {}
  }

  function handleUpdateHook(hk) {
    let cps = composeService.value
    let hooks = cps.hooks
    if (!cps.hooks) {
        hooks = []
    }

    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    if (index >= 0) {
      hooks.splice(index, 1)
    }
    
    hooks.push(hk)
    cps.hooks = hooks
    composeService.value = cps
  }

  function handleRemoveHook(hk) {
    let cps = composeService.value
    let hooks = cps.hooks
    if (!cps.hooks) {
        hooks = []
    }
    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    hooks.splice(index, 1)
    cps.hooks = hooks
    composeService.value = cps
  }

  function handleModifyHook(hk) {
    currentHook.value = hk
    showHookDialog.value = true
  }

  
  onMounted(() => {
      if (props.data && props.data.protocol && props.data.name) {
        var ns = route.query.ns as string
        fetchMetadata(props.data.protocol)
        fetchConfig(props.data.protocol, ns, props.data.name)
      }
      fetchAuthRoles()
      fetchLang()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  .el-select .el-input {
    width: 130px;
  }
  </style>
  