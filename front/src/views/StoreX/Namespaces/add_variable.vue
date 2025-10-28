<template>
    <el-dialog
      v-model="props.visible"
      title="添加/编辑执行变量"
      width="760"
      align-center
      @close="onDialogClosed"
    >
      <el-form :model="hook" label-width="100px" :inline="true" style="max-width: 600px">
            <el-form-item label="变量名称">
                <el-input v-model="hook.var_name" style="width: 600px" />
            </el-form-item>
            <el-form-item label="变量类型">
                <el-radio-group v-model="hook.var_type" style="width: 600px">
                    <el-radio-button value="string">String</el-radio-button>
                    <el-radio-button value="number">Number</el-radio-button>
                    <el-radio-button value="datetime">DateTime</el-radio-button>
                    <el-radio-button value="boolean">Boolean</el-radio-button>
                </el-radio-group>
            </el-form-item>
            <el-form-item label="变量当前值">
                <el-input type="textarea" v-model="hook.var_value" :rows="6" style="width: 600px" />
            </el-form-item>
            <el-form-item label="更新方式">
                <el-radio-group v-model="hook.var_write" style="width: 600px">
                    <el-radio-button value="MAX">MAX</el-radio-button>
                    <el-radio-button value="MIN">MIN</el-radio-button>
                    <el-radio-button value="CURRENT_DATE">当前日期</el-radio-button>
                    <el-radio-button value="CURRENT_DATETIME">当前日期时间</el-radio-button>
                    <el-radio-button value="SQL">SQL</el-radio-button>
                    <el-radio-button value="INVOKEURI">InvokeURI</el-radio-button>
                </el-radio-group>
            </el-form-item>
            <el-form-item v-if="hook.var_write !== 'CURRENT_DATETIME' && hook.var_write !== 'CURRENT_DATE'" label="表达式">
                <el-input type="textarea" v-model="hook.var_data_express" :rows="3" style="width: 600px" placeholder="请输入对应的表达式，更新方式为MAX/MIN时输入需要取值的JSONPath；为SQL时填写获取该变量SQL语句，为InvokeURI时，填写获取该变量的InvokeURI"/>
            </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="$emit('update:visible', false)">取消</el-button>
          <el-button type="primary" @click="onConfirm">
            确认
          </el-button>
        </div>
      </template>
    </el-dialog>
  </template>
  
  <script lang="ts" setup name="config">
  import { lang_list, update } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import { mergeProps, onMounted, ref, watch } from "vue";
  const props = defineProps<{ visible: boolean, hook: any }>();
  const emit = defineEmits(['update:visible', 'update:hook', 'datasync'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const ScriptLangs = ref<Array<any>>([])

  function handleLangList() {  
    lang_list().then(res => {
      ScriptLangs.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  }
  
  function onDialogClosed() {
    emit('update:visible', false)
  }
  
  function handleSelectionChange(e: any) {
    selections.value = e
  }
  
  function onConfirm() {
    emit('update:visible', false)
    emit('update:hook', props.hook)
  }
  
  onMounted(() => {
    handleLangList()
  });
  </script>
  