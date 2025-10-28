<template>
  <div class="dashboard-editor-container">
    <div class="topbar">
      <el-button type="warning" icon="odometer" @click="monitorSwitch">性能监控-{{ monitor ? '停止': '开始' }}</el-button>
      <span class="about" @click="showAboutMore">关于 v{{ verinfo.version }}</span>
    </div>
    <panel-group :count-data="countData" @handleSetLineChartData="handleSetLineChartData" />
    <el-row style="background:#fff;margin-bottom:8px;">
      <connection-summary :chart-data="connections" />
    </el-row>    
    <el-row style="background:#fff;margin-bottom:8px;">
      <invoke-uri-summary :chart-data="summary" />
    </el-row>
    <el-row style="margin-bottom:8px; display: flex; flex-direction: row; justify-content: space-between;">
      <el-col :span="8" style="background:#fff;">
        <cpu-line-chart :chart-data="lineChartData" />
      </el-col>
      <el-col :span="8" style="background:#fff;">
        <mem-line-chart :chart-data="lineChartData" />
      </el-col>
      <el-col :span="8" style="background:#fff;">
        <task-line-chart :chart-data="lineChartData" />
      </el-col>
    </el-row>
    <el-row style="background:#fff;margin-bottom:8px;">
      <line-chart :chart-data="lineChartData" />
    </el-row>
    <el-dialog
      v-model="showmore_dlg"
      :title="'关于' + verinfo.display_name + ' v' + verinfo.version"
      width="600"
      align-center
      :close-on-click-modal="false"
      :close-on-press-escape="false"      
      class="lic-dialog"
      @close="hideAboutMore">
      <div class="aboutcontent">
        <div class="about">{{ verinfo.about }}</div>
        <div class="about"><span class="title">版本 </span> {{ verinfo.version }}</div>
        <div class="about"><span class="title">CPU架构 </span> {{ verinfo.arch }}，<span class="title">核心数量 </span> {{ verinfo.cpu_cores }}</div>
        <div class="about"><span class="title">系统家族 </span> {{ verinfo.family }}</div>
        <div class="about"><span class="title">操作系统 </span> {{ verinfo.os_long_version }} 内核({{ verinfo.kernel_long_version }})</div>
        <div class="about"><span class="title">作者 </span> {{ verinfo.author }}</div>
      </div>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="primary" @click="hideAboutMore">确定</el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts" setup name="home">
import { onMounted, onUnmounted, ref, reactive } from "vue";
import { performance_get, performance_summary, version_info_get } from "@/http/modules/performance";
import PanelGroup from '../Dashboard/PanelGroup.vue'
import LineChart from '../Dashboard/LineChart.vue'
import CpuLineChart from '../Dashboard/CpuLineChart.vue'
import MemLineChart from '../Dashboard/MemLineChart.vue'
import TaskLineChart from '../Dashboard/TaskLineChart.vue'
import InvokeUriSummary from '../Dashboard/InvokeUriSummary.vue'
import ConnectionSummary from '../Dashboard/ConnectionSummary.vue'

const monitor = ref<boolean>(false)
const monitor_type = ref("success")
const interval = ref<any>(0)
const countData = ref<any>({ now_cpu_time: 0, diff_time: 1, memory_total: 0, cpu_cores: 0, threads: 0, counter: { task_1_count: 0, task_2_count: 0, task_3_count: 0, task_4_count: 0, task_5_count: 0 }})
const lineChartData = ref<any>({ data: [], title: '当前性能' });
const summary = ref<Array<any>>([]);
const connections = ref<Array<any>>([]);
const verinfo = ref<any>({});
const showmore_dlg = ref<boolean>(false)


function showAboutMore() {
  showmore_dlg.value = true
}

function hideAboutMore() {
  showmore_dlg.value = false
}

function handleSetLineChartData(type: any) {
      const ttl = '近一个月的' + (type === 'users' ? '用户数' : (type === 'questions' ? '提问数' : (type === 'reply' ? '回复数' : (type === 'meeting' ? '约见数' : '收入额'))))
}

function fetch_performance() {
  performance_get().then((res: any) => {
    if (res && res.status === 200) {
      let diff_ = (res.data.timestamp - countData.value.timestamp) / 1000.0;
      console.log('Timediff: ', diff_);
      let conns = res.data.connections
      let newt = { ...res.data,
                      network_recv_total: (res.data.network_recv_total - countData.value.network_recv_total) / diff_,
                      network_send_total: (res.data.network_send_total - countData.value.network_send_total) / diff_,
                      disk_read_total: (res.data.disk_read_total - countData.value.disk_read_total) / diff_,
                      disk_write_total: (res.data.disk_write_total - countData.value.disk_write_total) / diff_, 
                      now_cpu_time: res.data.now_cpu_time - countData.value.now_cpu_time, 
                      kernel_cpu_usages: res.data.kernel_cpu_usages /** - countData.value.kernel_cpu_usages **/, 
                      user_cpu_usages: res.data.user_cpu_usages /** - countData.value.user_cpu_usages **/, 
                      diff_time: diff_, 
                      prev_counter: countData.value.counter }
      countData.value = res.data
      if (lineChartData.value.data.length > 100) {
        lineChartData.value.data.shift()
      }
      lineChartData.value.data.push(newt)
      var connlist = []
      for (var k in conns) {
        connlist.push({
          db_url: k,
          ...conns[k]
        })
      }
      connlist.sort((a, b) => {
        return a.db_url.localeCompare(b.db_url)
      })
      connections.value = connlist
    }
  }).catch((ex: any) => {
    console.log(ex)
  })
}

function fetch_performance_summary() {
  performance_summary().then((res: any) => {
    if (res && res.status === 200) {
      summary.value = res.data
    }
  }).catch((ex: any) => {
    console.log(ex)
  })
}

function fetch_version_info() {
  version_info_get().then((res: any) => {
    if (res && res.status === 200) {
      verinfo.value = res.data
    }
  }).catch((ex: any) => {
    console.log(ex)
  })
}

function timed_fetch_performance() {
  if (interval.value) {
    clearInterval(interval.value)
  }
  interval.value = setInterval(() => {
    fetch_performance()
    fetch_performance_summary()
  }, 5000)
}

function monitorSwitch() {
  if (monitor.value) {
    clearInterval(interval.value)
    interval.value = 0
    monitor.value = false
    monitor_type.value = "success"
  } else {
    monitor.value = true
    monitor_type.value = "warning"
    timed_fetch_performance()
  }
}

onMounted(() => {
  fetch_version_info()
  fetch_performance()
  fetch_performance_summary()
});

onUnmounted(() => {
  if (interval.value) {
    clearInterval(interval.value)
    interval.value = 0
  }
});
</script>

<style lang="scss" scoped>
@use "index.scss";
.topbar {
  display: flex;
  justify-content: space-between;
  .about {
    cursor: pointer;
  }
}
.aboutcontent {
  padding: 20px;
}
.aboutcontent .about {
  line-height: 30px;
}
.aboutcontent .about .title {
  font-weight: 600;
}
</style>
