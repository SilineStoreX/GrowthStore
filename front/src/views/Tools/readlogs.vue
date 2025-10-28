<template>
  <div class="home">
    <div class="label_titel">
      <el-form :model="logview" label-width="100px" :inline="true" style="max-width: 100%; padding: 0px; height: 36px;">
        <el-form-item label="预读取行数">
          <el-input-number v-model="logview.reserves" :min="0" :max="5000" />
        </el-form-item>
        <el-form-item label="读取文件">
        <el-radio-group
          v-model="logview.logfile"
          text-color="#626aef"
          fill="rgb(239, 240, 253)"
        >
          <el-radio-button label="日志" value="logs" />
          <el-radio-button label="Trace" value="trace" />
          <el-radio-button label="nohup.log" value="nohup" />
        </el-radio-group>
        </el-form-item>
        <el-form-item label="刷新间隔(秒)">
          <el-input-number v-model="logview.interval" :min="1" :max="30" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="doRefreshLogs">刷新日志</el-button>
        </el-form-item>
      </el-form>
      <el-dropdown trigger="click"  @command="handleCommand" style="padding-top: 10px; height: 24px;">
        <span class="el-dropdown-link">
          归档日志下载
          <el-icon class="el-icon--right">
            <arrow-down />
          </el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item v-for="it in logslist" :key="it" :command="it">{{ it }}</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
    <div id="jsonContainer" class="editor">
      <MdPreview editorId="packed_id" :modelValue="logs" :codeFoldable="false" :showCodeRowNumber="true" />
    </div>
  </div>
</template>

<script lang="ts" setup name="form-pro">
import {ref, onMounted, onUnmounted} from 'vue'
import { getUrl } from '@/http/axios_api';
import { GlobalStore } from "@/stores";
import { MdPreview, MdCatalog } from 'md-editor-v3';
// preview.css相比style.css少了编辑器那部分样式
import 'md-editor-v3/lib/preview.css';
import { performance_logs, download_logs } from '@/http/modules/performance';

const globalStore = GlobalStore();
// 编辑器内容
const code = ref('')
const options = ref<any>({
  minimap: {
    enabled: false
  },
  automaticLayout: true
})

const logview = ref({interval: 3, logfile: "logs", reserves: 100});
const activeNames = ref("0");
const template = ref("")
const sourcejson = ref("");
const result = ref("");
const screenWidth = ref(1400);
const halfScreen = ref(690);
const logslist = ref([])
const logs = ref('```json\n');
var pre_reader = null

const fetchLogsList = () => {
  performance_logs().then(res => {
    logslist.value = res.data
  }).catch(ex => {
    console.log(ex)
  })
}

const doRefreshLogs = () => {
  const logparams = logview.value
  const baseurl = getUrl()
  const requrl = baseurl + "/management/performance/readlogs/sse?interval=" + logparams.interval + '&reserves=' + logparams.reserves + "&logfile=" + logparams.logfile
  const decoder = new TextDecoder();
  logs.value = "```json\n"
  if (pre_reader) {
    pre_reader.cancel("close").then(res => {
      console.log(res)
    })
  }
  fetch(requrl, {
      method: 'GET',
      headers: {
          'Authorization': get_auth_token(),
          'Content-Type': 'application/json'
      },
  }).then(res => {
    const reader = res.body.getReader();
    pre_reader = reader;
    reader.read().then(function processText(rt: any) {
        if (rt.done) {
            return;
        }
        var text = decoder.decode(rt.value);
        for (var line of text.split('\n')) {
            if (line.startsWith('data:')) {
                var data = line.substring('data:'.length)
                processOnMessage(data)
            } else {
                // 应该是调用出错了，才会来到这里
                try {
                    var ts = JSON.parse(text)
                    processOnMessage(ts);
                } catch (e) {
                }
            }
        }
        return reader.read().then(processText);
    }).catch((e: any) => {
        return;
    })
  }).catch(ex => {
    console.log(ex)
  })
}

function get_auth_token() {
    return "Bearer " + globalStore.token
}

function processOnMessage(text: string) {
  console.log(text)
  if (text.length > 0) {
    if (text.indexOf('\n') >= 0) {
      logs.value = logs.value + ' ' + text 
    } else {
      logs.value = logs.value + '\n' + text 
    }
  }
}

function handleCommand(e) {
  console.log(e)
  if (e && e != '') {
    download_logs(e).then(res => {
        let part = res as any;
        var blob = new Blob([part], {
            type: 'application/oct-stream;charset=utf-8'
        })
        const a = document.createElement('a')
        const URL = window.URL || window.webkitURL
        const href = URL.createObjectURL(blob)
        a.href = href
        a.download = e
        document.body.appendChild(a)
        a.click()
        document.body.removeChild(a)
        URL.revokeObjectURL(href)
    }).catch(ex => {
      console.log(ex)
    })
  }
}

onMounted(() => {
  fetchLogsList()
});

onUnmounted(() => {

})
</script>
<style lang="scss" scoped>
@use "index.scss";
$--el-main-padding: 0px;
</style>