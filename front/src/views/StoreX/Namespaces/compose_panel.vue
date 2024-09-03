<template>
    <div class="container">
        <add-hook :visible="showHookDialog" :hook="currentHook" @update:visible="handleHookDialogVisibleChange" @update:hook="handleUpdateHook" />
        <el-scrollbar>
            <el-form label-width="100px" :inline="false">
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
            </el-form>
            <el-divider />
            <template v-if="editingOrAdding !== 0">
                <el-form ref="composeEditFormRef" :model="composeService" label-width="140">
                    <el-form-item label="名称" prop="name">
                        <el-input v-model="composeService.name"></el-input>
                    </el-form-item>
                    <el-form-item label="脚本语言"  prop="lang">
                        <el-radio-group v-model="composeService.lang">
                            <el-radio-button key="shell" value="shell">Shell</el-radio-button>
                            <el-radio-button v-for="item in  ScriptLangs" :key="item.lang" :value="item.lang">{{ item.description }}</el-radio-button>
                        </el-radio-group>
                        <el-form-item label="返回类型"  prop="return_type">
                          <el-radio-group v-model="composeService.return_type">
                              <el-radio-button value="Nothing">无</el-radio-button>
                              <el-radio-button value="Single">单对象</el-radio-button>
                              <el-radio-button value="List">列表</el-radio-button>
                              <el-radio-button value="Page">分页列表</el-radio-button>
                          </el-radio-group>
                      </el-form-item>
                    </el-form-item>
                    <el-form-item v-if="composeService.lang === 'shell'" label=" ">
                      <span class="warn">*注意：Shell只能用于定时器任务</span>
                    </el-form-item>
                    <el-form-item label="发布为REST API"  prop="rest_api">
                        <el-switch v-model="composeService.rest_api" />
                        <el-form-item v-if="composeService.rest_api" label="文件上传服务"  prop="fileupload">
                          <el-switch v-model="composeService.fileupload" />
                          <el-input v-if="composeService.fileupload" v-model="composeService.file_field" placeholder="上传文件字段名" style="margin-left:10px; width: 120px"/>
                        </el-form-item>
                        <el-form-item label="定时器任务"  prop="schedule_on">
                          <el-switch v-model="composeService.schedule_on" />
                        </el-form-item>
                    </el-form-item>
                    <el-form-item v-if="composeService.rest_api" label="功能权限">
                      <el-select v-model="composeService.perm_roles" multiple collapse-tags collapse-tags-tooltip placeholder="选择赋予该功能可访问的角色" style="width: 240px">
                        <el-option value="" label="全部角色">全部角色</el-option>
                        <el-option v-for="r in auth_roles" :key="r" :value="r" :label="r">{{ r }}</el-option>
                      </el-select>
                      <el-form-item label="允许匿名访问">
                        <el-switch v-model="composeService.bypass_permission" />
                      </el-form-item>
                    </el-form-item>
                    <el-form-item v-if="composeService.schedule_on"  prop="cron_express" label="CRON表达式">
                        <el-input v-model="composeService.cron_express" placeholder="定时器任务的CRON表达式" style="width: 240px"/>
                        <el-form-item v-if="composeService.schedule_on && composeService.lang !== 'shell'"  prop="schedule_simulate" label="模拟登录用户">
                          <el-input v-model="composeService.schedule_simulate" placeholder="模拟登录用户帐号" />
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="同步队列">
                      <el-switch v-model="composeService.enable_synctask" />
                      <el-form-item v-if="composeService.enable_synctask" label="任务名称">
                        <el-input v-model="composeService.task_id" style="width: 110px"/>
                      </el-form-item>
                      <el-form-item v-if="composeService.enable_synctask" label="执行删除">
                        <el-switch v-model="composeService.execute_delete" />
                      </el-form-item>
                      <el-form-item v-if="composeService.enable_synctask && !composeService.execute_delete" label="删除标识">
                        <el-input v-model="composeService.check_delete" style="width: 110px"/>
                      </el-form-item>
                  </el-form-item>                    
                    <el-form-item label="脚本"  prop="script">
                        <el-input type="textarea" v-model="composeService.script" rows="12"/>
                    </el-form-item>
                    <el-form-item label="Hook"  prop="hooks">
                        <el-table :data="composeService.hooks">
                            <el-table-column type="expand">
                                <template #default="h">
                                    <code>
                                        {{ h.row.script }}
                                    </code>
                                </template>
                            </el-table-column>
                            <el-table-column prop="lang" label="脚本语言" />
                            <el-table-column prop="event" label="事件" />
                            <el-table-column prop="before" label="前置处理" />
                            <el-table-column label="操作" width="100px">
                                <template #default="scoped">
                                    <el-button type="primary" icon="Edit" circle @click="handleModifyHook(scoped.row)" />
                                    <el-popconfirm title="确认要删除吗?" @confirm="handleRemoveHook(scoped.row)">
                                        <template #reference>
                                            <el-button type="danger" icon="Delete" circle />
                                        </template>
                                    </el-popconfirm>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button @click="onAddHook">添加</el-button>
                    </el-form-item>
                </el-form>
                <div style="margin-top: 20px">
                    <el-button type="primary" @click="onSaveEditComposeService">保存服务</el-button>
                    <el-button @click="onCancelReturn">返回</el-button>
                </div>                
            </template>
            <template v-else>
                <el-table :data="config_data">
                    <el-table-column prop="name" label="名称" />
                    <el-table-column prop="lang" label="脚本语言" />
                    <el-table-column prop="rest_api" label="REST-API" />
                    <el-table-column prop="schedule_on" label="定时任务" />
                    <el-table-column prop="return_type" label="返回值" />
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

  const props = defineProps<{ data: any }>();
  const emit = defineEmits(['update:data', 'update:visible'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const activeName = ref<any>("query")
  const protocol_forms = ref<Array<any>>([])
  const config_data = ref<Array<any>>([])
  const editingOrAdding = ref<any>(0)
  const composeService = ref<any>({})
  const ScriptLangs = ref<Array<any>>([])
  const composeEditFormRef = ref<FormInstance>()
  const showHookDialog = ref<boolean>(false)
  const currentHook = ref<any>()
  const auth_roles = ref<Array<any>>([])

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
      config_save(props.data.protocol, ns, props.data.name, { services: config_data.value }).then(res => {
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
      config_data.value = res.data.services
    }).catch(ex => {
      console.log(ex)
    })
  }

  function saveConfig(schema: string, ns: string, name: string) {
    config_save(schema, ns, name, config_data.value).then(res => {
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
  @import "index.scss";
  </style>
  