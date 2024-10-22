<template>
  <div class="form">
    <el-row style="line-height: 50px;">
      <el-col :colspan="24">
        <div class="label_titel"><span>Tear模板在线测试工具</span><el-button type="primary" @click="doJsonPathTest">测试</el-button></div>
        <div id="jsonContainer" class="editor">
          <span>输入的需要被测试的模板</span>
          <MonacoEditor
            theme="dark"
            language="plain"
            :options="options"
            :height="300"
            :diffEditor="false"
            v-model:value="template"
          ></MonacoEditor>
        </div>
      </el-col>
    </el-row>
    <el-row>
      <el-col :colspan="24">
        <el-collapse v-model="activeNames">
          <el-collapse-item title="Tera模板说明" name="2">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>Tera函数</th>
                  <th>描述</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>jsonpath</td>
                  <td>使用JSONPath的规范来取值，传入的参数必须为JSON对象，JSONPath的使用可以参考“JSONPath表达式说明”。示例：<span> &lbrace;&lbrace; jsonpath(arg=args[0], path="$.data") &rbrace;&rbrace; </span></td>
                </tr>
                <tr>
                  <td>to_json</td>
                  <td>将JSON对象转换成为JSON String表示。示例：<span>&lbrace;&lbrace; to_json(value = args[0]) &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>sha1_text</td>
                  <td>计算文本的SHA1的哈希值，算法是sha1_256。示例：<span>&lbrace;&lbrace; sha1_text(text = args[0]) &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>sha2_text</td>
                  <td>计算文本的SHA2的哈希值，算法是sha2_256。示例：<span>&lbrace;&lbrace; sha2_text(text = args[0]) &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>md5string</td>
                  <td>计算文本的MD5的哈希值。示例：<span>&lbrace;&lbrace; md5string(text = args[0]) &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>hmac_sha1</td>
                  <td>使用HMACSHA1算法加密。示例：<span>&lbrace;&lbrace; hmac_sha1(key = "thisisakey", text = "valuetobehmachash") &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>hmac_sha256</td>
                  <td>使用HMACSHA256算法加密。示例：<span>&lbrace;&lbrace; hmac_sha256(key = "thisisakey", text = "valuetobehmachash") &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>hmac_sha512</td>
                  <td>使用HMACSHA512算法加密。示例：<span>&lbrace;&lbrace; hmac_sha512(key = "thisisakey", text = "valuetobehmachash") &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>base64_encode</td>
                  <td>执行BASE64编码处理。示例：<span>&lbrace;&lbrace; base64_encode(text = "valuetobehmachash") &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>base64_decode</td>
                  <td>执行BASE64解码处理。示例：<span>&lbrace;&lbrace; base64_decode(text = "dGhpc2lzYWtleXM=") &rbrace;&rbrace;</span></td>
                </tr>
                <tr>
                  <td>canonicalized_query</td>
                  <td>对指定的JSON对象的Key值进行排序，并按此顺序生成QueryString，常用于支付宝等接口的调用。示例：<span>&lbrace;&lbrace; canonicalized_query(value = args[0]) &rbrace;&rbrace;</span></td>
                </tr>
              </tbody>
            </table>
            <a href="https://keats.github.io/tera/docs/">查看Tera模板基础语法和使用规范 - https://keats.github.io/tera/docs/</a>
          </el-collapse-item>
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
            <span>输入参数（JSON格式表示）</span>
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
            <span>模板解析结果</span>
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
import { tera_test } from '@/http/modules/tools'
// 编辑器内容
const code = ref('')
const options = ref<any>({
  minimap: {
    enabled: false
  },
  automaticLayout: true
})

const activeNames = ref("0");
const template = ref("")
const sourcejson = ref("");
const result = ref("");
const screenWidth = ref(1400);
const halfScreen = ref(690);

const doJsonPathTest = () => {
  console.log(template.value, sourcejson.value);
  var tvs = {}
  try {
    tvs = JSON.parse(sourcejson.value)
  }catch(e) {}

  tera_test(template.value, tvs).then(res => {
    if (res.status === 0 || res.status === 200) {
      result.value = res.data
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