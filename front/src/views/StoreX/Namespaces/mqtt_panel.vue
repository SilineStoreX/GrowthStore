<template>
    <div class="container">
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
                  <el-collapse-item title="MQTT连接配置" name="info">                
                    <el-form-item label="连接地址">
                        <el-input v-model="rest_conf.connection" />
                    </el-form-item>
                    <el-form-item label="用户名">
                        <el-input v-model="rest_conf.username" />
                    </el-form-item>
                    <el-form-item label="密码">
                        <el-input v-model="rest_conf.password" />
                    </el-form-item>
                    <el-form-item label="Keep Alive">
                        <el-input v-model="rest_conf.keep_alive" />
                    </el-form-item>
                    <el-form-item label="自动重连时间">
                        <el-input v-model="rest_conf.min_retry" style="width: 270px"/>
                        <el-form-item label="最大重试时间">
                            <el-input v-model="rest_conf.max_retry" style="width: 270px"/>
                        </el-form-item>
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider />
            <template v-if="editingOrAdding !== 0">
                <el-form ref="composeEditFormRef" :model="composeService" label-width="140">
                    <el-form-item label="主题" prop="topic">
                        <el-input v-model="composeService.topic" style="width: 290px"></el-input>
                        <el-form-item label="QoS" prop="qos">
                            <el-select v-model="composeService.qos" style="width: 290px">
                              <el-option label="0" :value="0">0</el-option>
                              <el-option label="1" :value="1">1</el-option>
                              <el-option label="2" :value="2">2</el-option>                              
                            </el-select>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="描述" prop="rest_desc">
                        <el-input v-model="composeService.description"></el-input>
                    </el-form-item>
                    <el-form-item label="消费者">
                      <el-switch v-model="composeService.consumer" />
                      <el-form-item label="生产者">
                        <el-switch v-model="composeService.producer" />
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
                    <el-form-item label="脚本语言"  prop="lang">
                        <el-radio-group v-model="composeService.lang">
                            <el-radio-button v-for="item in  ScriptLangs" :key="item.lang" :value="item.lang">{{ item.description }}</el-radio-button>
                        </el-radio-group>
                    </el-form-item>
                    <el-form-item label="消费处理脚本"  prop="script">
                        <el-input type="textarea" v-model="composeService.script" rows="12"/>
                    </el-form-item>
                </el-form>
                <div style="margin-top: 20px">
                    <el-button type="primary" @click="onSaveEditComposeService">保存服务</el-button>
                    <el-button @click="onCancelReturn">返回</el-button>
                </div>
            </template>
            <template v-else>
                <el-table :data="config_data">
                    <el-table-column prop="topic" label="主题" width="180px" />
                    <el-table-column prop="qos" label="QoS" witdh="60px"/>
                    <el-table-column prop="consumer" label="消费者" witdh="60px"/>
                    <el-table-column prop="producer" label="生产者" witdh="60px"/>
                    <el-table-column prop="lang" label="脚本类型" witdh="100px"/>
                    <el-table-column prop="description" label="描述" :show-overflow-tooltip="true" />
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
  
  <script lang="ts" setup name="mqtt">
  import { update, remove, metadata_get, config_get, config_save, lang_list, authorize_roles_get } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import { VxeUI, VxeFormPropTypes, VxeFormEvents } from 'vxe-table'
  import { mergeProps, onMounted, ref, watch } from "vue";
  import { FormInstance } from "element-plus";

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
      rest_conf.value = res.data
      config_data.value = res.data.services
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
  