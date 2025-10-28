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
                  <el-collapse-item title="语音处理参数配置" name="info">                
                    <el-form-item label="模型存储路径">
                        <el-input v-model="rest_conf.model_path" />
                    </el-form-item>
                    <el-form-item label="语音识别模型">
                        <el-input v-model="rest_conf.voice_recogniza_model" />
                    </el-form-item>
                    <el-form-item label="语音识别线程数">
                        <el-input v-model="rest_conf.num_threads" />
                    </el-form-item>
                    <el-form-item label="语音识别引擎">
                        <el-select v-model="rest_conf.using_recognization">
                            <el-option value="zipformer" label="ZipFormer">ZipFormer</el-option>
                            <el-option value="whisper"  label="Whiper">Whisper</el-option>
                            <el-option value="paraformer"  label="ParaFormer">ParaFormer</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="TTS引擎">
                        <el-select v-model="rest_conf.tts_engine">
                            <el-option value="vits" label="Vites TTS">Vites TTS</el-option>
                            <el-option value="kokoro"  label="Kokoro TTS">Kokoro TTS</el-option>
                        </el-select>
                    </el-form-item>
                    <el-form-item label="TTS模型">
                        <el-input v-model="rest_conf.tts_model" />
                    </el-form-item>
                    <el-form-item label="Speaker Id">
                        <el-input v-model="rest_conf.speaker_id" placeholder="请参照具体模型的说明来填写"/>
                    </el-form-item>
                    <el-form-item label="TTS生成语音存放路径">
                        <el-input v-model="rest_conf.store_path" />
                    </el-form-item>
                    <el-form-item label="HTTP访问的前缀">
                        <el-input v-model="rest_conf.access_url_prefix" />
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
            </el-form>
            <el-divider />
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
  
  <script lang="ts" setup name="voice">
  import { update, remove, metadata_get, config_get, config_save, lang_list, authorize_roles_get } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import { mergeProps, onMounted, ref, watch } from "vue";
  import { ElMessageBox, FormInstance } from "element-plus";

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
          ElMessageBox.alert('保存失败', "提示", { type: 'warning' })
        }
      }).catch(me => {
        ElMessageBox.alert('保存失败', "提示", { type: 'warning' })
      })

    }).catch(ex => {
      ElMessageBox.alert('保存插件信息失败' + ex.description, "提示", { type: 'warning' })
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
  