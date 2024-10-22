<template>
  <el-container class="form-container" ref="computedContainer">
    <el-row style="line-height: 50px;">
      <el-col :colspan="24">
        <div class="label_titel"><span>Rhai脚本在线测试工具</span><el-button type="primary" @click="doJsonPathTest">测试</el-button></div>
        <div id="jsonContainer" class="editor">
          <span class="fullline">输入的需要被被测试的Rhai脚本</span>
          <span class="label">脚本的返回类型</span>
          <el-radio-group v-model="return_type">
              <el-radio-button value="Single">单对象</el-radio-button>
              <el-radio-button value="List">列表</el-radio-button>
              <el-radio-button value="Page">分页列表</el-radio-button>
          </el-radio-group>
          <MonacoEditor
            theme="vs"
            language="rust"
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
          <el-collapse-item title="Rhai脚本语言说明" name="2">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th width="200">Rhai函数或指令</th>
                  <th>描述</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>required</td>
                  <td>创建GrowthStore的内置对象或服务，如存储服务，查询服务，以及插件服务，参数是传入InvokeURI。示例：<pre> 
                    let st = required("object://com.siline/User"); 
                    let val = st.select(ctx, 0);
                  </pre></td>
                </tr>
                <tr>
                  <td>存储对象服务</td>
                  <td>使用required创建，其InvokeURI表示为：object://[namespace]/[name]，然后可以使用select, find_one, query, paged_query, update, insert, upsert, save_batch, delete, delete_by, update_by等方法。这些方法的第一个参数必须为ctx，表示执行上下文，第二个参数为对应方法的功能性参数，如select方法，的第二个参数可能是String,或i64，表示ID值；而其它的方法的第二个参数为JSON对象数组。
                    如果，方法需要使用多个参数的，都是使用JSON对象数组来传递。如update, insert, delete，这几个方法的第二个参数（JSON数组）的第一个对象即为被处理的JSON对象，它们不会处理该参数中的第二个对象。find_one, query, paged_query, delete_by传入的参数的第一个对象为QueryCondition，用于检索；upsert, update_by中第二个参数中的第一个对象的为操作的数据，第二个对象为QueryCondition。
                    save_batch方法中的第二个参数的数组为将要被保存的数据对象数组，这些数据会在一个事务里执行保存（upsert）。<br/>
                    存储对象服务还提供了与namespace相关联的几个加密解密算法。如aes_encrypt/aes_decrypt（使用AES算法进行加密解密），rsa_encrypt/rsa_decrypt （使用RSA算法进行加密解密），其中的密钥等信息存储在该namespace中的相关配置中。这几个方法的调用不需要传递ctx上下文对象。
                  </td>
                </tr>
                <tr>
                  <td>查询对象服务</td>
                  <td>使用required创建，其InvokeURI表示为：query://[namespace]/[name]。可以使用search或paged_search方法来进行查询，前者返回列表，后者返回分页列表。其中， 第一个参数为ctx，第二个参数为JSON对象数组。在第二个参数中，第一个对象的用于查询对象的固定参数的传递，第二个对象为QueryCondition。paged_search中，第二个参数的QueryCondition中需要设置其中的分页信息（paging: { size: 10, current: 1 }）</td>
                </tr>
                <tr>
                  <td>插件服务</td>
                  <td>使用required创建，其InvokeURI表示为：[plugin_protocol]://[namespace]/[name]。可以使用invoke_return_option（返回单个或JSON对象）、invoke_return_vec（返回JSON对象数组）、invoke_return_page（返回分页表示的JSON对象，注：通常除了compose外的插件，不会使用这个方法）三个其本方法来进行查询。其中， 第一个参数为ctx，第二个参数为插件所定义的方法名称，第三个参数为JSON对象数组。在第三个参数中，为对应方法所能处理的参数，具体的定义需要与具体的方法所定义参数能力进行匹配。
                    插件服务中所自定义的方法可以直接用其名称来调用，这个时候，则只需要传递ctx，以及功能的参数即可。
                    <pre>
                      let pst = required("restapi://com.siline/Alipay");
                      pst.payment(ctx, args);
                    </pre>
                  </td>
                </tr>
                <tr>
                  <td>ctx</td>
                  <td>函数调用时所使用的上下文。可以使用get("arg_name") 获取值，返回JSON对象，get_bool(arg_name)获取BOOL值，get_i64(arg_name)获取i64值，get_u64(arg_name)获取u64值，get_string(arg_name)获取String值，get_hook_uri()获取当前被HOOK的URI（只在HOOK处理中有效），set(arg_name, value)设置值。set_return(ret_value)设置返回值。get_return()获取返回值。</td>
                </tr>
                <tr>
                  <td>JSON对象</td>
                  <td>
                    <pre>
to_string, 转换成JSON String,
is_paged, 是否为分页对象
is_list, 是否为列表
is_empty, 是否为空
len 数组的长度
push，往数组的尾部插入JSON对象
select，入参为JSONPath表达式，返回该JSONPath表示的JSON对象，如果返回为空，则返回对象的is_empty为真。
get_records，对于分页对象，获取分页对象中的记录列表
set_records(vec)，对于分页对象，获取分页对象中的记录列表
get_page_no，对于分页对象，获取分页对象当前页
set_page_no(no)，对于分页对象，设置分页对象当前页
get_total，对于分页对象，获取分页对象中中总记录数
set_total(total)，对于分页对象，设置分页对象中总记录数
get_page_size，对于分页对象，获取分页对象的每页数量
set_page_size(size)，对于分页对象，设置分页对象每页数量
[id/name]: 使用索引的方法来取值，如果是列表，则按对应序号来获得相应的对象。如果JSON对象，则按其Key来获取其值。
to_rhai_object，转换成为Rhai的Map对象，这样可以更方法的在Rhai脚本中使用；
to_rhai_array，转换成为Rhai的数组对象，这样可以更方法的在Rhai脚本中使用；
to_array，将单个对象转换成为数组列表；
canonicalized_query(arg): 对指定的JSON对象的Key值进行排序，并按此顺序生成QueryString，常用于支付宝等接口的调用。
unwrap(): 由于返回的JSON对象有多种形态，使用该方法统一转换为一个形态。
new_json_object：创建一个空的JSON对象，或者从Rhai对象中进行创建 let json = new_json_object(); 或 let json = new_json_object(#{});
new_json_null(): 创建一个Null值的JSON对象；
new_json_array(): 创建一个空的JSON对象数组，可以使用push方法往里面添加对象。
new_json_string(text)：通过String来创建一个JSON对象；
new_json_bool(bl)：通过bool来创建一个JSON对象；
new_json_number(num)：通过Number来创建一个JSON对象；
new_json_paged(records: Vec[Value],total: i64,page_no: i64,page_size: i64): 根据参数创建一个分页JSON对象。上述参数都为可选。
new_query_condition(and: Value,or: Value,g: Value,ord: Value,page: Value): 创建一个QueryCondition对象。参数均可选。
pub struct QueryCondition {
    pub and: Vec[ConditionItem],
    pub or: Vec[ConditionItem],
    pub sorts: Vec[OrdianlItem],
    pub group_by: Vec[OrdianlItem],
    pub paging: Option[IPaging],
}
new_ordianl_item(): 创建一个OrdianlItem对象
pub struct OrdianlItem {
    pub field: String,
    pub sort_asc: bool,
}
new_condition_item(): 创建一个ConditionItem对象
pub struct ConditionItem {
    pub field: String,
    pub op: String,
    pub value: Value,
    pub value2: Value,
    pub and: Vec[ConditionItem],
    pub or: Vec[ConditionItem],
}

                    </pre>
                  </td>
                </tr>
                <tr>
                  <td>DateTime 函数</td>
                  <td><pre>
now()：当前日期时间
now_utc()：当前日期时间UTC；
datetime()：当前日期时间
datetime_utc()：当前日期时间UTC；
to_string()：转换为String
to_global(): 转换为Global的String
to_locale()：转换为Locale的String
to_rfc2822()：转换为rfc2822的格式的String
format(fmt)：格式化，参数可选
parse(text, fmt)：解析字符串表示的日期为指定格式。fmt为可选。
year()：日期时间中的年
month()：日期时间中的月
day()：日期时间中的日
hour()：日期时间中的小时
minute()：日期时间中的分钟
second()：日期时间中的秒
timestamp_second()：时间戳的秒表示（从1970年开始）
timestamp_micro()：时间戳的微秒表示（从1970年开始）
timestamp_millis()：时间戳的毫秒表示（从1970年开始）
                  </pre></td>
                </tr>
                <tr>
                  <td>HTTP操作</td>
                  <td>
                    <pre>
http_request(uri: string, method: Method, data: Value, opt: Value):  执行URI的HTTP请求。
http_get(uri: string, data: Value, opt: Value):  执行URI的HTTP GET请求，data和opt可选。
http_post(uri: string, data: Value, opt: Value):  执行URI的HTTP POST请求，opt可选。
http_put(uri: string, data: Value, opt: Value):  执行URI的HTTP PUT请求，opt可选。
http_delete(uri: string, data: Value, opt: Value):  执行URI的HTTP DELETE请求，opt可选。
http_option(uri: string, opt: Value):  执行URI的HTTP OPTION请求，opt可选。
http_connect(uri: string, opt: Value):  执行URI的HTTP CONNECT请求，opt可选。
http_head(uri: string, opt: Value):  执行URI的HTTP HEAD请求，opt可选。
http_patch(uri: string, opt: Value):  执行URI的HTTP PATCH请求，opt可选。
                    </pre>
                  </td>
                </tr>
                <tr>
                  <td>其它公共函数</td>
                  <td>
                    <pre>
sha1_text(text): 对字符串获取其sha1哈希值
sha2_text(text): 对字符串获取其sha2哈希值
hmac_sha1(key, text): 使用HMACSHA1算法加密。
hmac_sha256(key, text): 使用HMACSHA256算法加密。
hmac_sha512(key, text): 使用HMACSHA512算法加密。
md5string(text): 对字符串获取其MD5哈希值
base64encode(text|blob): 对字符串或二进制数组执行Base64编码
base64decode(text|blob): 对字符串或二进制数组执行Base64编码
snowflake_id([])：返回雪花ID，不传任何参数，则使用系统缺省的参数来调用。否则根据指定的参数来构建雪花ID。
uuid(): 返回UUID字符串
                    </pre>
                  </td>
                </tr>
              </tbody>
            </table>
            <a href="https://rhai.rs/book/">查看Rhai脚本语言基础语法和使用规范 - https://rhai.rs/book/</a>
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
              theme="vs"
              language="json"
              :options="options"
              :height="400"
              :diffEditor="false"
              v-model:value="sourcejson"
            ></MonacoEditor>
          </div>
          <div id="resultContainer" class="editor">
            <span>脚本执行结果</span>
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
  </el-container>
</template>

<script lang="ts" setup name="form-pro">
import * as monaco from 'monaco-editor';
import {ref, onMounted, onUnmounted, onBeforeUnmount} from 'vue'
import MonacoEditor from 'monaco-editor-vue3'
import { rhai_test } from '@/http/modules/tools'
// 编辑器容器div

  // 编辑器内容
const code = ref('')
const options = ref<any>({
  minimap: {
    enabled: false
  },
  automaticLayout: true
})

const computedContainer = ref(null);
const activeNames = ref("0");
const template = ref("")
const sourcejson = ref("");
const result = ref("");
const return_type = ref("Single")

const doJsonPathTest = () => {
  console.log(template.value, sourcejson.value);
  var tvs = {}
  try {
    tvs = JSON.parse(sourcejson.value)
  }catch(e) {}

  rhai_test(template.value, return_type.value, tvs).then(res => {
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

onBeforeUnmount(() => {
  
});

onUnmounted(() => {

})
</script>
<style lang="scss" scoped>
@import "index.scss";
</style>