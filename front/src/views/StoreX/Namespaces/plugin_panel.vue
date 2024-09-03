<template>
    <div class="container">
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
            <vxe-form
              v-if="data.enable"
              :data="config_data"
              :items="protocol_forms"
              titleColon
              title-align="right"
              title-width="160"
              @submit="submitEvent"
              @reset="resetEvent">
              <template #myregion="{ data }">
                <vxe-input v-model="data.region" placeholder="自定义插槽模板"></vxe-input>
              </template>
            </vxe-form>
        </el-scrollbar>
      <div class="drawer-footer">
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
  import { update, remove, metadata_get, config_get, config_save } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import { VxeUI, VxeFormPropTypes, VxeFormEvents } from 'vxe-table'
  import { mergeProps, onMounted, ref, watch } from "vue";
  const props = defineProps<{ data: any }>();
  const emit = defineEmits(['update:data', 'update:visible'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const activeName = ref<any>("query")
  const protocol_forms = ref<Array<any>>([])
  const config_data = ref<any>({})

  watch(
    () => [props.data.protocol, props.data.name],
    (newVal, oldVal) => {
      console.log('Watch for props ', newVal, oldVal)
      var ns = route.query.ns as string
      fetchMetadata(newVal[0])
      fetchConfig(newVal[0], ns, newVal[1])
    }
  )
  
  function handleUpdate() {
    var ns = route.query.ns as string
    update(ns, 'plugin', [props.data]).then(_res => {
      config_save(props.data.protocol, ns, props.data.name, config_data.value).then(res => {
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
      config_data.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  }

  function saveConfig(schema: string, ns: string, name: string) {
    config_save(schema, ns, name, config_data.value).then(res => {
      config_data.value = res.data
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
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  