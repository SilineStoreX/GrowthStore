<template>
    <el-dialog
      v-model="props.visible"
      title="添加/编辑执行变量"
      width="800"
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
            <el-form-item label="变更更新方式">
                <el-input type="textarea" v-model="hook.var_write" :rows="6" style="width: 600px" placeholder="可以使用CURRENT_DATE代表当前日期，CURRENT_DATETIME代表当前日期时间，或者填写SQL来执行查询，或者填写InvokeURI来执行预先定义好的查询，当使用SQL查询或InvokeURI时，返回的结果最好只有一个字段，或者只包含一个以_value结尾的字段作为该变更最终的更新值。"/>
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
  