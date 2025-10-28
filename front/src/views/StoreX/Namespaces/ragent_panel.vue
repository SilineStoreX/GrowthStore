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
                  <el-collapse-item title="RAgent配置" name="info">
                    <el-divider>大语言模型参数</el-divider>
                    <el-form-item label="大模型提供者">
                        <el-select v-model="rest_conf.llm_provider">
                          <el-option v-for="it in model_data" :key="it.provider_id" :value="it.provider_id" :label="it.provider_id">{{ it.provider_id }}</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="模型名称">
                        <el-input v-model="rest_conf.llm_model" />
                    </el-form-item>                    
                    <el-form-item label="Temperature">
                        <el-input v-model="rest_conf.temperature" />
                    </el-form-item>
                    <el-form-item label="Top P">
                        <el-input v-model="rest_conf.top_p" />
                    </el-form-item>
                    <el-form-item label="最大Token数">
                        <el-input v-model="rest_conf.max_tokens" />
                    </el-form-item>
                    <el-form-item label="频率惩罚">
                        <el-input v-model="rest_conf.frequency_penalty" />
                    </el-form-item>                    
                    <el-divider>RAG模型参数</el-divider>  
                    <el-form-item label="Embedding提供者">
                        <template #label>
                            Embedding提供者
                            <el-tooltip class="box-item" effect="dark" placement="top-start" content="目前Embedding只支持OpenAI及兼容接口和Ollama的提供者">
                              <el-icon><InfoFilled /></el-icon>
                            </el-tooltip>
                        </template>
                        <el-select v-model="rest_conf.embedding_provider">
                          <el-option v-for="it in model_data" :key="it.provider_id" :value="it.provider_id" :label="it.provider_id">{{ it.provider_id }}</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="Embedding模型">
                        <el-input v-model="rest_conf.embedding_model" />
                    </el-form-item>
                    <el-form-item label="Embedding维度">
                        <el-input v-model="rest_conf.embedding_dims" />
                    </el-form-item>
                    <el-form-item label="ReRank提供者">
                        <el-select v-model="rest_conf.rerank_provider">
                          <el-option v-for="it in model_data" :key="it.provider_id" :value="it.provider_id" :label="it.provider_id">{{ it.provider_id }}</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="ReRank模型">
                        <el-input v-model="rest_conf.rerank_model" />
                    </el-form-item>
                    <el-form-item label="ReRank采样">
                        <el-input v-model="rest_conf.rerank_sampling" />
                    </el-form-item>
                    <el-form-item label="RAG提供者URI">
                        <el-input v-model="rest_conf.rag_provider_uri" />
                    </el-form-item>
                    <el-form-item label="RAG相似度">
                        <el-input v-model="rest_conf.rag_similarity" />
                    </el-form-item>
                    <el-form-item label="RAG检索数量">
                        <el-input v-model="rest_conf.rag_fetch_count" />
                    </el-form-item>
                    <el-divider>提示词配置</el-divider>
                    <el-form-item label="提示词提供者URI">
                        <el-input v-model="rest_conf.prompt_provider_uri" />
                    </el-form-item>
                    <el-form-item label="缺省提示词URI">
                        <el-input v-model="rest_conf.prompt_default_uri" />
                    </el-form-item>
                    <el-form-item label="提示词相似度">
                        <el-input v-model="rest_conf.prompt_similarity" />
                    </el-form-item>
                    <el-form-item label="提示词最大检索数量">
                        <el-input v-model="rest_conf.prompt_fetch_count" />
                    </el-form-item>
                    <el-divider>MCP工具配置</el-divider>
                    <el-form-item label="MCP工具检索URI">
                        <template #label>
                          MCP工具检索URI
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="MCP服务的连接、调用等使用插件rag进行统一管理，这里填写插件rag公开出来的InvokeURI">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="rest_conf.mcp_endpoint" />
                    </el-form-item>
                    <el-form-item label="MCP工具调用URI">
                        <template #label>
                          MCP工具调用URI
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="MCP服务的连接、调用等使用插件rag进行统一管理，这里填写插件rag公开出来的InvokeURI">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="rest_conf.toolcall_endpoint" />
                    </el-form-item>
                    <el-form-item label="MCP相似度大于">
                        <template #label>
                          MCP相似度大于
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="在Growth AI的RAgent中，为了防止及减少不相关的工具调用，对MCP工具也使用相似度查询">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="rest_conf.mcp_score_threshold" />
                    </el-form-item>
                    <el-form-item label="MCP检索数量">
                        <template #label>
                          MCP检索数量
                          <el-tooltip class="box-item" effect="dark" placement="top-start" content="最多返回的MCP工具数量">
                            <el-icon><InfoFilled /></el-icon>
                          </el-tooltip>
                        </template>
                        <el-input v-model="rest_conf.mcp_fetch_count" />
                    </el-form-item>
                    <el-divider>记忆系统参数</el-divider>
                    <el-form-item label="记忆提供者URI">
                        <el-input v-model="rest_conf.memory_provider_uri" />
                    </el-form-item>
                    <el-form-item label="短时记忆提供者URI">
                        <el-input v-model="rest_conf.short_memory_provider_uri" />
                    </el-form-item>
                    <el-form-item label="记忆管理URI">
                        <el-input v-model="rest_conf.memory_managed_uri" />
                    </el-form-item>
                    <el-form-item label="记忆相似度">
                        <el-input v-model="rest_conf.memory_similarity" />
                    </el-form-item>
                    <el-form-item label="记忆历史检索数量">
                        <el-input v-model="rest_conf.memory_fetch_count" />
                    </el-form-item>
                    <el-divider>意图识别模型</el-divider>
                    <el-form-item label="意图类型提供者">
                        <el-input v-model="rest_conf.intent_provider_uri" />
                    </el-form-item>
                    <el-form-item label="训练数据提供者">
                        <el-input v-model="rest_conf.training_example_provider_uri" />
                    </el-form-item>
                    <el-form-item label="特征尺寸">
                        <el-input v-model="rest_conf.feature_dimensions" />
                    </el-form-item>
                    <el-form-item label="最大语汇表大小">
                        <el-input v-model="rest_conf.max_vocabulary_size" />
                    </el-form-item>
                    <el-form-item label="最小置信度">
                        <el-input v-model="rest_conf.min_confidence_threshold" />
                    </el-form-item>
                    <el-form-item label="重新训练阀值">
                        <el-input v-model="rest_conf.retraining_threshold" />
                    </el-form-item>                    
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider>模型提供者管理</el-divider>
            <template v-if="modelEditingOrAdding !== 0">
                <el-form ref="modelEditFormRef" :model="composeModel" label-width="140">
                    <el-form-item label="提供者标识" prop="provider_id">
                        <el-input v-model="composeModel.provider_id"></el-input>
                    </el-form-item>
                    <el-form-item label="类型" prop="provider_type">
                        <el-select v-model="composeModel.provider_type">
                          <el-option value="openai" label="OpenAI及兼容接口">OpenAI兼容接口</el-option>
                          <el-option value="deepseek" label="DeepSeek">DeepSeek</el-option>
                          <el-option value="siliconflow" label="硅基流动">硅基流动</el-option>
                          <el-option value="zhipu" label="智普AI">智普AI</el-option>
                          <el-option value="doubao" label="豆包">豆包</el-option>
                          <el-option value="ollama" label="Ollama">Ollama</el-option>
                          <el-option value="gemini" label="Gemini">Gemini</el-option>
                          <el-option value="anthropic" label="Anthropic">Anthropic</el-option>
                          <el-option value="groq" label="GroQ">GroQ</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="服务地址" prop="endpoint">
                        <el-input v-model="composeModel.endpoint"></el-input>
                    </el-form-item>
                    <el-form-item label="App Key" prop="appkey">
                        <el-input v-model="composeModel.appkey"></el-input>
                    </el-form-item>
                </el-form>
                <div style="margin-top: 20px">
                    <el-button type="primary" @click="onSaveEditComposeModel">保存提供者</el-button>
                    <el-button @click="onCancelReturn">返回</el-button>
                </div>
            </template>
            <template v-else>
                <el-table :data="model_data">
                    <el-table-column prop="provider_id" label="标识" width="180px" />
                    <el-table-column prop="provider_type" label="提供者" :show-overflow-tooltip="true" width="100px" />
                    <el-table-column prop="endpoint" label="服务地址" witdh="150px"/>
                    <el-table-column label="操作" width="100px">
                        <template #default="scoped">
                            <el-button type="primary" icon="Edit" circle @click="onEditComposeModel(scoped.row)" />
                            <el-popconfirm title="确认要删除吗?" @confirm="onDeleteComposeModel(scoped.row)">
                                <template #reference>
                                    <el-button type="danger" icon="Delete" circle />
                                </template>
                            </el-popconfirm>
                        </template>
                    </el-table-column>
                </el-table>
                <div style="margin-top: 20px">
                    <el-button @click="onAddComposeModel">添加</el-button>
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
  const model_data = ref<Array<any>>([])
  const rest_conf = ref<any>({})
  const editingOrAdding = ref<any>(0)
  const modelEditingOrAdding = ref<any>(0)
  const composeService = ref<any>({})
  const composeModel = ref<any>({})
  const ScriptLangs = ref<Array<any>>([])
  const composeEditFormRef = ref<FormInstance>()
  const modelEditFormRef = ref<FormInstance>()
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
      config_save(props.data.protocol, ns, props.data.name, { ...restconf, tools: [], models: model_data.value, services: config_data.value }).then(res => {
        if (res.status === 0 || res.status === 200) {
          emit("update:visible", false)
          emit("update:data", true)
        } else {
          ElMessageBox.alert('保存失败', "提示", { type: 'warning' })
        }
      }).catch(me => {
        ElMessageBox.alert('保存失败', "提示", { type: 'warning' })
      })

    }).catch(ex => {
      ElMessageBox.alert('保存插件信息失败', "提示", { type: 'warning' })
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
      model_data.value = res.data && res.data.models ? res.data.models : []
    }).catch(ex => {
      console.log(ex)
    })
  }

  function __saveConfig(schema: string, ns: string, name: string) {
    config_save(schema, ns, name, config_data.value).then(res => {
      rest_conf.value = res.data || {}
      config_data.value = res.data && res.data.services ? res.data.services : []
      model_data.value = res.data && res.data.models ? res.data.models : []
    }).catch(ex => {
      console.log(ex)
    })
  }

  

  function onAddComposeModel() {
    modelEditingOrAdding.value = 1
    composeModel.value = {}
  }

  function onAddComposeService() {
    editingOrAdding.value = 1
    composeService.value = {}
  }

  function onCancelReturn() {
    composeEditFormRef.value?.resetFields()
    modelEditFormRef.value?.resetFields()
    editingOrAdding.value = 0
    modelEditingOrAdding.value = 0
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

  function onSaveEditComposeModel() {
    if (modelEditingOrAdding.value === 1) {
        var composes = model_data.value
        composes.push(composeModel.value)
        model_data.value = composes
    }
    modelEditingOrAdding.value = 0
  }

  function onDeleteComposeModel(raw) {
    var composes = model_data.value
    let index = composes.indexOf(raw) // 找到要删除的元素的索引，此处为 2
    composes.splice(index, 1)
    model_data.value = composes
  }


  function onEditComposeModel(raw) {
    modelEditingOrAdding.value = 2
    composeModel.value = raw 
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

  // const submitEvent: VxeFormEvents.Submit = () => {
  //   console.log("config: ", config_data.value)
  //   VxeUI.modal.message({ content: '保存成功', status: 'success' })
  // }

  // const resetEvent: VxeFormEvents.Reset = () => {
  //   VxeUI.modal.message({ content: '重置事件', status: 'info' })
  // }
  
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
  