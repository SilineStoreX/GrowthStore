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
                  <el-collapse-item title="MCP配置" name="info">
                    <el-form-item label="MCP协议版本">
                      <el-select v-model="rest_conf.protocol_version" placeholder="MCP服务协议版本" style="width: 240px">
                        <el-option value="2024-11-05" label="2024-11-05">2024-11-05</el-option>
                        <el-option value="2025-03-26" label="2025-03-26">2025-03-26</el-option>
                      </el-select>
                    </el-form-item>
                    <el-form-item label="MCP服务介绍">
                        <el-input v-model="rest_conf.instructions" type="textarea" :rows="4" placeholder="MCP服务介绍，详细的服务介绍有助于MCP工具的选择。"/>
                    </el-form-item>
                    <el-form-item label="编录MCP命名空间">
                        <el-input v-model="rest_conf.list_namespaces" type="textarea" :rows="4" placeholder="输入需要进行编录的命名空间，多个命名空间之间使用半角分号分隔"/>
                    </el-form-item>
                    <el-form-item prop="interval_second" label="刷新时间">
                      <el-input v-model="rest_conf.interval_second" placeholder="刷新MCP服务内容的时间，每隔指定秒数后会进行一次MCP服务能力的刷新。" style="width: 270px" />
                    </el-form-item>
                    <el-form-item prop="enable_resource" label="状态管理">
                      <el-switch v-model="rest_conf.stateful_mode" />
                    </el-form-item>
                    <el-form-item prop="cursor_size" label="游标大小">
                      <template #label>
                          游标大小
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="也就是分页的大小，每次查出多少条记录，缺省为100条记录。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                      </template>
                      <el-input v-model="rest_conf.cursor_size" placeholder="当需要使用游标进行获取资源/提示词，工具时，每次返回的数量" style="width: 270px" />
                    </el-form-item>
                    <el-form-item prop="enable_all_cursor" label="全部启用游标">
                      <template #label>
                          全部启用游标
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="如果启用，则在工具查询/提示词查询和资源查询中启用游标，否则只有资源查询使用游标。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                      </template>
                      <el-switch v-model="rest_conf.enable_all_cursor" />
                    </el-form-item>
                    <el-form-item prop="enable_resource" label="启用资源">
                      <el-switch v-model="rest_conf.enable_resource" />
                    </el-form-item>
                    <el-form-item v-if="rest_conf.enable_resource" prop="enable_object_resources" label="对象转为资源">
                      <el-switch v-model="rest_conf.enable_object_resources" />
                    </el-form-item>
                    <el-form-item v-if="rest_conf.enable_resource" label="从InvokeURI中读取">
                        <el-input v-model="rest_conf.resource_invoke_uri" type="textarea" :rows="4" placeholder="输入需要从中读取资源的URI，多个URI之间使用半角分号分隔"/>
                    </el-form-item>
                    <el-form-item v-if="rest_conf.enable_resource" label="资源模板URI">
                        <el-input v-model="rest_conf.resource_template_uri" type="textarea" :rows="4" placeholder="输入需要从中读取资源的模板URI，多个URI之间使用半角分号分隔"/>
                    </el-form-item>
                    <el-form-item prop="enable_prompts" label="启用提示词">
                      <el-switch v-model="rest_conf.enable_prompts" />
                    </el-form-item>
                    <el-form-item v-if="rest_conf.enable_prompts" label="从InvokeURI中读取">
                      <el-input v-model="rest_conf.prompts_invoke_uri" type="textarea" :rows="4" placeholder="输入需要从中读取提示词的URI，多个URI之间使用半角分号分隔"/>
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
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
      ElMessageBox.alert('保存插件信息失败，' + ex.description, "提示", { type: 'warning' })
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
  