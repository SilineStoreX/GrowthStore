<template>
    <el-dialog
      v-model="props.visible"
      title="添加/编辑Hook"
      width="800"
      align-center
      @close="onDialogClosed"
    >
      <el-form :model="hook" label-width="100px" :inline="true" style="max-width: 600px">
            <el-form-item label="脚本语言">
                <el-radio-group v-model="hook.lang" style="width: 600px">
                    <el-radio-button value="invoke_uri">URI</el-radio-button>
                    <el-radio-button v-for="item in  ScriptLangs" :key="item.lang" :value="item.lang">{{ item.description }}</el-radio-button>
                </el-radio-group>                
            </el-form-item>
            <el-form-item label="前置Hook">
                <el-switch v-model="hook.before" />
                <el-form-item label="事件">
                    <el-switch v-model="hook.event" /><span>事件采用异步执行，不等待返回。</span>
                </el-form-item>
            </el-form-item>
            <el-form-item label="脚本">
                <el-input type="textarea" v-model="hook.script" rows="10" style="width: 600px" />
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
  