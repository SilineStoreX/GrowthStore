<template>
  <div class="dashboard-editor-container">
    <el-button type="warning" icon="odometer" @click="monitorSwitch">性能监控-{{ monitor ? '停止': '开始' }}</el-button>
    <panel-group :count-data="countData" @handleSetLineChartData="handleSetLineChartData" />
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
  </div>
</template>

<script lang="ts" setup name="home">
import { onMounted, onUnmounted, ref, reactive } from "vue";
import { performance_get, performance_summary } from "@/http/modules/performance";
import PanelGroup from '../Dashboard/PanelGroup.vue'
import LineChart from '../Dashboard/LineChart.vue'
import CpuLineChart from '../Dashboard/CpuLineChart.vue'
import MemLineChart from '../Dashboard/MemLineChart.vue'
import TaskLineChart from '../Dashboard/TaskLineChart.vue'
import InvokeUriSummary from '../Dashboard/InvokeUriSummary.vue'

const monitor = ref<boolean>(false)
const monitor_type = ref("success")
const interval = ref<any>(0)
const countData = ref<any>({ now_cpu_time: 0, diff_time: 1, memory_total: 0, cpu_cores: 0, threads: 0, counter: { task_1_count: 0, task_2_count: 0, task_3_count: 0, task_4_count: 0, task_5_count: 0 }})
const lineChartData = ref<any>({ data: [], title: '当前性能' });
const summary = ref<Array<any>>([]);

function handleSetLineChartData(type: any) {
      const ttl = '近一个月的' + (type === 'users' ? '用户数' : (type === 'questions' ? '提问数' : (type === 'reply' ? '回复数' : (type === 'meeting' ? '约见数' : '收入额'))))
}

function fetch_performance() {
  performance_get().then((res: any) => {
    if (res && res.status === 200) {
      let diff_ = (res.data.timestamp - countData.value.timestamp) / 1000.0;
      console.log('Timediff: ', diff_);
      let newt = { ...res.data,
                      network_recv_total: (res.data.network_recv_total - countData.value.network_recv_total) / diff_,
                      network_send_total: (res.data.network_send_total - countData.value.network_send_total) / diff_,
                      disk_read_total: (res.data.disk_read_total - countData.value.disk_read_total) / diff_,
                      disk_write_total: (res.data.disk_write_total - countData.value.disk_write_total) / diff_, 
                      now_cpu_time: res.data.now_cpu_time - countData.value.now_cpu_time, 
                      kernel_cpu_usages: res.data.kernel_cpu_usages - countData.value.kernel_cpu_usages, 
                      user_cpu_usages: res.data.user_cpu_usages - countData.value.user_cpu_usages, 
                      diff_time: diff_, 
                      prev_counter: countData.value.counter }
      countData.value = res.data
      if (lineChartData.value.data.length > 100) {
        lineChartData.value.data.shift()
      }
      lineChartData.value.data.push(newt)
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
  // timed_fetch_performance()
});

onUnmounted(() => {
  if (interval.value) {
    clearInterval(interval.value)
    interval.value = 0
  }
});
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
