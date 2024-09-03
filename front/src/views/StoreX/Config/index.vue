<template>
    <div class="home">
      <div>Upload test</div>
      <el-upload
        class="upload-demo"
        action="/api/compose/com.siline/stayhere/upload/upload"
        method="post"
        :on-preview="handlePreview"
        :on-remove="handleRemove"
        :before-remove="beforeRemove"
        multiple
        :data="{key: 'test'}"
        :limit="3"
        :on-exceed="handleExceed"
        :file-list="fileList"
      >
        <el-button size="small" type="primary">点击上传</el-button>
        <template #tip>
          <div class="el-upload__tip">只能上传 jpg/png 文件，且不超过 500kb</div>
        </template>
      </el-upload>
    </div>
  </template>
  
  <script lang="ts" setup name="config">
  import { ElMessage } from "element-plus";
  import { onMounted, ref } from "vue";
  const fileList = ref<Array<any>>([])

  function handleRemove(file, fileList) {
    console.log(file, fileList)
  }

  function handlePreview(file) {
        console.log(file)
  }

  function handleExceed(files, fileList) {
      ElMessage.warning(
        `当前限制选择 3 个文件，本次选择了 ${files.length} 个文件，共选择了 ${
          files.length + fileList.length
        } 个文件`
      )
  }
  
  function beforeRemove(file, fileList) {
    ElMessage(`确定移除 ${file.name}？`)
    return true
  }

  onMounted(() => {
    console.log("config");
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  