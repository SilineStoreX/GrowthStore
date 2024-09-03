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
                  <el-collapse-item title="REST API配置" name="info">                
                    <el-form-item label="AppId">
                        <el-input v-model="rest_conf.app_id" />
                    </el-form-item>
                    <el-form-item label="App Secret">
                        <el-input v-model="rest_conf.app_secret" />
                    </el-form-item>
                    <el-form-item label="API服务地址">
                        <el-input v-model="rest_conf.api_server" />
                    </el-form-item>
                    <el-form-item label="OAuth2验证">
                        <el-switch v-model="rest_conf.enable_oauth2" />
                        <el-form-item label="接受已失效证书">
                            <template #label>
                              接受已失效证书
                              <el-tooltip class="box-item" effect="dark" placement="top-start" content="启用后，将忽略对服务器端HTTPS证书的校验。只对HTTPS请求有效。">
                                <el-icon><InfoFilled /></el-icon>
                              </el-tooltip>
                            </template>
                            <el-switch v-model="rest_conf.accept_invalid_certs" />
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="OAuth2验证URL">
                        <el-input v-model="rest_conf.oauth2_validate_url" class="input-with-select">
                          <template #prepend>
                            <el-select v-model="rest_conf.oauth2_request_method" style="width: 100px">
                              <el-option label="GET" value="GET"></el-option>
                              <el-option label="POST" value="POST"></el-option>
                              <el-option label="PUT" value="PUT"></el-option>
                            </el-select>
                          </template>
                          <template #append>
                            <el-select v-model="rest_conf.oauth_request_type" style="width: 100px" placeholder="请求类型">
                              <el-option v-if="rest_conf.oauth2_request_method !== 'GET'" label="JSON" value="JSON"></el-option>
                              <el-option v-if="rest_conf.oauth2_request_method !== 'GET'" label="XML" value="XML"></el-option>
                              <el-option v-if="rest_conf.oauth2_request_method !== 'GET'" label="FORM" value="FORM"></el-option>
                              <el-option v-if="rest_conf.oauth2_request_method === 'GET'" label="QUERY" value="QUERY"></el-option>
                            </el-select>
                          </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="OAuth2验证模板">
                        <template #label>
                          OAuth2验证模板
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="采用模板方式来生成最终用于请求Token交换的接口。如要引用配置内容，则使用config.为前缀，如需要引用参数，则使用args[0|1].为前缀；如需要引用返回值，则以ret.为前缀。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>                      
                        <el-input v-model="rest_conf.oauth2_request_body" type="textarea" :rows="4" />
                    </el-form-item>
                    <el-form-item label="返回的Token字段">
                        <template #label>
                          返回的Token字段
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="使用JSON Path表达式来获取Token字段的值。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="rest_conf.oauth2_token_express" style="width: 40%"/>
                        <el-form-item label="返回的Expired">
                          <template #label>
                            返回的Expired
                            <el-tooltip class="box-item" effect="dark" placement="top-start" content="使用JSON Path表达式来获取expired字段的值。如果没有该值，缺省为72秒。">
                              <el-icon><InfoFilled /></el-icon>
                            </el-tooltip>
                          </template>
                          <el-input v-model="rest_conf.oauth2_expired_express"  style="width: 100%"/>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="接口传递Token方式">
                        <el-radio-group v-model="rest_conf.token_pass_style" style="width: 40%">
                          <el-radio-button label="Cookie" value="Cookie" />
                          <el-radio-button label="Header" value="Header" />
                          <el-radio-button label="Query" value="Query" />
                        </el-radio-group>                    
                        <el-form-item label="Token标识">
                          <template #label>
                            Token标识
                            <el-tooltip class="box-item" effect="dark" placement="top-start" content="用于向后续接口调用传递Access-Token的标识名称">
                              <el-icon><InfoFilled /></el-icon>
                            </el-tooltip>
                          </template>
                          <el-input v-model="rest_conf.token_identifier"  style="width: 100%"/>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="请求的自定义Header">
                        <el-input v-model="rest_conf.custom_headers" type="textarea" :rows="4" placeholder="请使用JSON结构表示自定义头，错误的JSON体将会被认为没有自定义头" />
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider />
            <template v-if="editingOrAdding !== 0">
                <el-form ref="composeEditFormRef" :model="composeService" label-width="140">
                    <el-form-item label="名称" prop="name">
                        <el-input v-model="composeService.name"></el-input>
                    </el-form-item>
                    <el-form-item label="接口描述" prop="rest_desc">
                        <el-input v-model="composeService.rest_desc"></el-input>
                    </el-form-item>
                    <el-form-item label="服务URL" prop="rest_url">
                        <el-input v-model="composeService.rest_url">
                          <template #prepend>
                            <el-select v-model="composeService.rest_method" style="width: 100px">
                              <el-option label="GET" value="GET"></el-option>
                              <el-option label="POST" value="POST"></el-option>
                              <el-option label="PUT" value="PUT"></el-option>
                              <el-option label="DELETE" value="DELETE"></el-option>
                            </el-select>
                          </template>
                          <template #append>
                            <el-select v-model="composeService.rest_content_type" style="width: 100px" placeholder="请求类型">
                              <el-option v-if="composeService.rest_method !== 'GET'" label="JSON" value="JSON"></el-option>
                              <el-option v-if="composeService.rest_method !== 'GET'" label="XML" value="XML"></el-option>
                              <el-option v-if="composeService.rest_method !== 'GET'" label="FORM" value="FORM"></el-option>
                              <el-option v-if="composeService.rest_method === 'GET'" label="QUERY" value="QUERY"></el-option>
                            </el-select>
                          </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="发布为REST API"  prop="rest_api">
                        <el-switch v-model="composeService.rest_api" />
                        <el-form-item label="不带Access Token"  prop="no_access_token">
                          <el-switch v-model="composeService.no_access_token" />
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
                    <el-form-item label="请求模板"  prop="script">
                        <template #label>
                          请求模板
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="采用模板方式来生成最终用于请求接口的参数。如要引用配置内容，则使用config.为前缀，如需要引用参数，则使用args[0|1].为前缀；如需要引用返回值，则以ret.为前缀。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input type="textarea" v-model="composeService.rest_body" rows="8"/>
                    </el-form-item>
                    <el-form-item label="返回验证"  prop="return_validate">
                        <template #label>
                          返回验证
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="对RESTful请求返回值进行验证，确保返回的结果是正确的。此处采用JSON Path规则描述。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="composeService.return_validate" placeholder="使用JSON Path提取成功与否并进行判断" style="width: 40%"></el-input>
                        <el-form-item label="返回数据"  prop="return_data">
                          <template #label>
                            返回数据
                            <el-tooltip class="box-item" effect="dark" placement="top-start" content="获取RESTful请求的返回值。此处采用JSON Path表达式提取数据。">
                              <el-icon><InfoFilled /></el-icon>
                            </el-tooltip>
                          </template>
                          <el-input v-model="composeService.return_data" placeholder="使用JSON Path提取数据" style="width: 100%"></el-input>
                      </el-form-item>
                    </el-form-item>
                    <el-form-item label="可用于登录"  prop="use_auth">
                        <template #label>
                          可用于登录
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="将该RESTful请求的结果，用于登录验证。通常用于从第三方用户身份接口中获得用户的标识后使用。该功能需要登录与认证中所配置的查找用户的URI配合使用。">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>                      
                        <el-switch v-model="composeService.use_auth" />
                        <el-form-item v-if="composeService.use_auth" label="验证标识"  prop="captcha_id_express">
                            <template #label>
                            验证标识
                            <el-tooltip class="box-item" effect="dark" placement="top-start" content="作为登录验证与该次请求进行关联的校验">
                              <el-icon><InfoFilled /></el-icon>
                            </el-tooltip>
                          </template>
                          <el-input v-model="composeService.captcha_id_express" style="width: 160px" placeholder="输入从参数中获取值的模板表达式"/>
                        </el-form-item>
                        <el-form-item v-if="composeService.use_auth" label="验证Code"  prop="captcha_code_express">
                          <el-input v-model="composeService.captcha_code_express" style="width: 160px" placeholder="输入从返回结果中获取值的模板表达式" />
                        </el-form-item>
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
                    <el-table-column prop="name" label="名称" width="180px" />
                    <el-table-column prop="rest_desc" label="描述" :show-overflow-tooltip="true" width="180px" />
                    <el-table-column prop="rest_method" label="请求" witdh="60px"/>
                    <el-table-column prop="rest_url" label="REST URL" :show-overflow-tooltip="true" width="180px"/>
                    <el-table-column prop="rest_api" label="发布" witdh="60px" />
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
  .el-select .el-input {
    width: 130px;
  }
  @import "index.scss";
  </style>
  