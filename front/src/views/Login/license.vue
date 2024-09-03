<template>
    <el-dialog
      v-model="props.visible"
      :title="props.title"
      width="1000"
      align-center
      :close-on-click-modal="false"
      :close-on-press-escape="false"      
      class="lic-dialog"
      @close="onDialogClosed"
    >
      <el-form :model="hook" :inline="true">
            <el-form-item>
                <iframe :src="props.src" style="height: calc(100vh - 180px); width: 960px; border: 1px;"></iframe>
            </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="primary" @click="$emit('update:visible', false)">确定</el-button>
        </div>
      </template>
    </el-dialog>
  </template>
  
  <script lang="ts" setup name="config">
  import { onMounted, ref } from "vue";
  const props = defineProps<{ visible: boolean, hook: any, title: string, src: string }>();
  const emit = defineEmits(['update:visible', 'update:hook', 'datasync'])
  const selections = ref<Array<any>>([])
  
  function onDialogClosed() {
    emit('update:visible', false)
  }
  
  onMounted(() => {
  });
  </script>
  <style lang="scss" scoped>
  .lic-dialog {
    height: calc(100vh - 40px);
  }
  :deep(.el-textarea textarea) {
    height: calc(100vh - 180px);
  }
  </style>