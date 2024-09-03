<template>
    <div class="container">
        <add-hook :visible="showHookDialog" :hook="currentHook" @update:visible="handleHookDialogVisibleChange" @update:hook="handleUpdateHook" />
        <add-variable :visible="showVariableDialog" :hook="currentVariable" @update:visible="handleVariableDialogVisibleChange" @update:hook="handleUpdateVariable" />
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
                  <el-collapse-item title="同步任务配置" name="info">                
                    <el-form-item label="最大写入线程数">
                        <el-input v-model="rest_conf.write_threads" />
                    </el-form-item>
                    <el-form-item label="执行变量定义"  prop="variables">
                        <el-table :data="rest_conf.variables">
                            <el-table-column prop="var_name" label="变量名称" width="160px"/>
                            <el-table-column prop="var_type" label="类型"  width="120px"/>
                            <el-table-column prop="var_value" label="当前值"  />
                            <el-table-column label="操作" width="100px">
                                <template #default="scoped">
                                    <el-button type="primary" icon="Edit" circle @click="handleModifyVariable(scoped.row)" />
                                    <el-popconfirm title="确认要删除吗?" @confirm="handleRemoveVariable(scoped.row)">
                                        <template #reference>
                                            <el-button type="danger" icon="Delete" circle />
                                        </template>
                                    </el-popconfirm>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button @click="onAddVariable">添加</el-button>
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider />
            <template v-if="editingOrAdding !== 0">
                <el-form ref="composeEditFormRef" :model="composeService" label-width="140">
                    <el-form-item prop="task_id">
                        <template #label>
                          任务名称
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="该字段修改后，需要重启才能停止原来的同步任务。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="composeService.task_id"></el-input>
                    </el-form-item>
                    <el-form-item label="任务描述" prop="task_desc">
                        <el-input v-model="composeService.task_desc"></el-input>
                    </el-form-item>
                    <el-form-item label="无源数据请求" prop="no_source">
                      <template #label>
                          无源数据请求
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="启用该选项意味着所有的源数据来自于事件源（如MQTT或Kafka等）">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>                      
                        <el-switch v-model="composeService.no_source" />
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="CRON表达式" prop="cron_express">
                        <el-input v-model="composeService.cron_express"></el-input>
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="源数据请求URI" prop="source_uri">
                        <template #label>
                          源数据请求URI
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="该字段修改后，需要重启才能停止原来的同步任务。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="composeService.source_uri"></el-input>
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="检查删除标识">
                        <el-input v-model="composeService.check_delete" placeholder="如果通过该标识检查，则认为该条记录应该执行删除操作"/>
                      </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="源数据请求" prop="source_request">
                        <el-input v-model="composeService.source_request" type="textarea" placeholder="请求时需要提供的参数表达式模板，通过该模板产生用于执行《源数据请求URI》的参数（JSON表示）"></el-input>
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="分页请求"  prop="paged_request">
                        <el-switch v-model="composeService.paged_request" />
                        <span>启用分页请求后，可以在参数中使用${page_no}和${page_size}</span>
                        <el-form-item v-if="composeService.paged_request" label="每页请求记录数" prop="page_size">
                            <el-input v-model="composeService.page_size" style="width: 90px"></el-input>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="转换脚本" prop="transform">
                        <el-input v-model="composeService.transform" type="textarea" placeholder="将源数据请求URI获得的数据作为参数，转换为写入的URI可以接受的数据格式"></el-input>
                    </el-form-item>
                    <el-form-item v-if="!composeService.no_source" label="变量更新时机">
                        <el-switch v-model="composeService.update_variable_epoch" />
                        启用后将在每次执行后更新变量，否则只在获取到最新数据后才进行更新。
                    </el-form-item>
                    <el-form-item label="写入的URI" prop="target_uri">
                        <el-input v-model="composeService.target_uri"></el-input>
                    </el-form-item>
                    <el-form-item label="删除的URI" prop="remove_uri">
                        <el-input v-model="composeService.remove_uri"></el-input>
                    </el-form-item>
                </el-form>
                <div style="margin-top: 20px">
                    <el-button type="primary" @click="onSaveEditComposeService">保存服务</el-button>
                    <el-button @click="onCancelReturn">返回</el-button>
                </div>
            </template>
            <template v-else>
                <el-table :data="config_data">
                    <el-table-column prop="task_id" label="名称" width="180px" />
                    <el-table-column prop="task_desc" label="描述" :show-overflow-tooltip="true" width="180px" />
                    <el-table-column prop="source_uri" label="源数据" witdh="60px"/>
                    <el-table-column prop="paged_request" label="分页请求" witdh="60px" />                    
                    <el-table-column prop="target_uri" label="写入" :show-overflow-tooltip="true" width="180px"/>
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
  import { VxeUI, VxeFormPropTypes, VxeFormEvents } from 'vxe-table'
  import { mergeProps, onMounted, ref, watch } from "vue";
  import { FormInstance } from "element-plus";
  import AddHook from "./add_hook.vue"
  import AddVariable from "./add_variable.vue"

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
  const showVariableDialog = ref<boolean>(false)  
  const currentHook = ref<any>()
  const currentVariable = ref<any>()
  const auth_roles = ref<Array<any>>([])
  const activeNames = ref<Array<any>>([])
  

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
          VxeUI.modal.message({ content: '保存失败', status: 'info' })
        }
      }).catch(me => {
        VxeUI.modal.message({ content: '保存失败, ' + me.description, status: 'info' })        
      })

    }).catch(ex => {
      VxeUI.modal.message({ content: '保存插件信息失败, ' + ex.description, status: 'info' })
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
      rest_conf.value = res.data || []
      config_data.value = res.data?.services
    }).catch(ex => {
      console.log(ex)
    })
  }

  function saveConfig(schema: string, ns: string, name: string) {
    config_save(schema, ns, name, config_data.value).then(res => {
      rest_conf.value = res.data
      config_data.value = res.data.services
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

  function handleVariableDialogVisibleChange(e: any) {
    showVariableDialog.value = e
  }  

  function onAddVariable() {
    showVariableDialog.value = true
    currentVariable.value = {}
  }

  function handleRemoveVariable(hk) {
    let cps = rest_conf.value
    let vars = cps.variables
    if (!cps.variables) {
        vars = []
    }
    let index = vars.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    vars.splice(index, 1)
    cps.variables = vars
    rest_conf.value = cps
  }

  function handleModifyVariable(hk) {
    currentVariable.value = hk
    showVariableDialog.value = true
  }

  function handleUpdateVariable(hk) {
    let cps = rest_conf.value
    let vars = cps.variables
    if (!cps.variables) {
        vars = []
    }

    let index = vars.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    if (index >= 0) {
      vars.splice(index, 1)
    }
    
    vars.push(hk)
    cps.variables = vars
    rest_conf.value = cps
  }

  const submitEvent: VxeFormEvents.Submit = () => {
    console.log("config: ", config_data.value)
    VxeUI.modal.message({ content: '保存成功', status: 'success' })
  }

  const resetEvent: VxeFormEvents.Reset = () => {
    VxeUI.modal.message({ content: '重置事件', status: 'info' })
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
  .el-select .el-input {
    width: 130px;
  }
  @import "index.scss";
  </style>
  