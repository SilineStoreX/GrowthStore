<template>
  <div class="form">
    <el-row style="line-height: 50px;">
      <el-col :colspan="24">
        <div class="label_titel">JSONPath 在线测试工具</div>
        <el-input v-model="jsonpath" placeholder="输入JSONPath表达式"  style="font-size: 20px; line-height: 40px; height: 50px;">
          <template #append>
            <el-button @click="doJsonPathTest">测试</el-button>
          </template>
        </el-input>
      </el-col>
    </el-row>
    <el-row>
      <el-col :colspan="24">
        <el-collapse v-model="activeNames">
          <el-collapse-item title="JSONPath表达式说明" name="1">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>JSONPath</th>
                  <th>描述</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>$</td>
                  <td>根对象/元素</td>
                </tr>
                <tr>
                  <td>@</td>
                  <td>当前对象/元素</td>
                </tr>
                <tr>
                  <td>. or []</td>
                  <td>取子对象/元素操作</td>
                </tr>
                <tr>
                  <td>..</td>
                  <td>递归向下取子对象/元素。JSONPath借鉴了E4X的语法。</td>
                </tr>
                <tr>
                  <td>*</td>
                  <td>通配符。所有对象/元素，无论其名称如何。</td>
                </tr>
                <tr>
                  <td>[]</td>
                  <td>下标运算符。XPath使用它来迭代元素集合和谓词。在Javascript和JSON中，它是原生数组运算符。</td>
                </tr>
                <tr>
                  <td>[,]</td>
                  <td>XPath中的联合运算符导致节点集的组合。JSONPath允许将备用名称或数组索引作为一个集合。</td>
                </tr>
                <tr>
                  <td>[start:end:step]</td>
                  <td>数组切片运算符，借用了ES4。</td>
                </tr>
                <tr>
                  <td>?()</td>
                  <td>应用筛选器（脚本）表达式。</td>
                </tr>
                <tr>
                  <td>()</td>
                  <td>脚本表达式，使用底层脚本引擎。</td>
                </tr>
              </tbody>
            </table>
            <a href="https://goessner.net/articles/JsonPath/index.html#e2">查看JSONPath表达式 - https://goessner.net/</a>
          </el-collapse-item>
        </el-collapse>
      </el-col>
    </el-row>
    <el-row>
      <el-col :colspan="24">
        <div class="split">
          <div id="editorContainer" class="editor">
            <span>输入的JSON文档</span>
            <MonacoEditor
              theme="dark"
              language="json"
              :options="options"
              :height="400"
              :diffEditor="false"
              v-model:value="sourcejson"
            ></MonacoEditor>
          </div>
          <div id="resultContainer" class="editor">
            <span>匹配结果</span>
            <MonacoEditor
              theme="vs"
              language="json"
              :options="options"
              :height="400"
              :diffEditor="false"
              v-model:value="result"
            ></MonacoEditor>
          </div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script lang="ts" setup name="form-pro">
import * as monaco from 'monaco-editor';
import {ref, onMounted, onUnmounted} from 'vue'
import MonacoEditor from 'monaco-editor-vue3'
import { jsonpath_test } from '@/http/modules/tools'
// 编辑器容器div

  // 编辑器内容
const code = ref('')
const options = ref<any>({
  minimap: {
    enabled: false
  },
  automaticLayout: true
})

const activeNames = ref("0");

const jsonpath = ref("");
const sourcejson = ref("");
const result = ref("");
const screenWidth = ref(1400);
const halfScreen = ref(690);

const doJsonPathTest = () => {
  console.log(jsonpath.value, sourcejson.value);
  var tvs = {}
  try {
    tvs = JSON.parse(sourcejson.value)
  }catch(e) {}

  jsonpath_test(jsonpath.value, tvs).then(res => {
    if (res.status === 0 || res.status === 200) {
      result.value = JSON.stringify(res.data, null, '\t')
    } else {
      result.value = JSON.stringify(res, null, '\t')
    }
  }).catch(ex => {
    result.value = JSON.stringify(ex, null, '\t')
  })
}

onMounted(() => {
  
});

onUnmounted(() => {

})
</script>
<style lang="scss" scoped>
@import "index.scss";
</style>