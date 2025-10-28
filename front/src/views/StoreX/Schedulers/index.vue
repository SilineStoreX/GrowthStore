<template>
  <div class="home">
    <div class="title">
      <span>定时器任务及长时间任务状态</span>
    </div>
    <el-tabs v-model="activeName" @tab-click="onTabPanelClick">
      <el-tab-pane label="周期性任务" name="repeatable">
        <div class="form">
          <span class="item">
            自动刷新<el-switch v-model="auto_refresh" @change="startFetchInterval"></el-switch>
            每<el-input-number v-model="refresh_seconds" :disabled="!auto_refresh" :min="1" :max="20" style="width: 120px;" @change="startFetchInterval"/>秒
            <el-button type="primary" @click="startFetchInterval" style="margin-left: 20px;">刷新</el-button>
          </span>
        </div>
        <el-table :data="tasksList" ref="redistable" :expand-row-keys="expandedkeys" row-key="uuid" @expand-change="onTableRowExpanded">
          <el-table-column type="expand">
              <template #default="scoped">
                  <span>执行参数</span>
                  <JsonViewer :value="scoped.row.params" :expand-depth="5" copyable boxed sort></JsonViewer>
              </template>
          </el-table-column>          
          <el-table-column prop="uuid" label="UUID" width="240px" show-overflow-tooltip />
          <el-table-column prop="invoke_uri" label="任务URI"  width="240px" show-overflow-tooltip />
          <el-table-column prop="states" label="状态" />
          <el-table-column prop="sched_type" label="类型">
            <template #default="scoped">
              {{ scoped.row.sched_type === 0 ? '一次性':  scoped.row.sched_type === 1 ? '重复性': 'CRON' }}
            </template>
          </el-table-column>
          <el-table-column prop="sched_express" label="定时器" />
          <el-table-column prop="description" label="任务描述" width="240px" show-overflow-tooltip />
          <el-table-column prop="execute_time" label="上次执行时间" width="160px">
            <template #default="scoped">
              {{ scoped.row.execute_time ? new Date(scoped.row.execute_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
            </template>            
          </el-table-column>
          <el-table-column prop="elapsed" label="耗时(微秒)">
            <template #header>
              <el-tooltip class="box-item" effect="dark" placement="top-start" content="该任务上次执行的所耗时间，单位微秒">
                <span>耗时(微秒)<el-icon><InfoFilled /></el-icon></span>
              </el-tooltip>
            </template>            
          </el-table-column>
          <el-table-column prop="times" label="次数" width="80px">
            <template #header>
              <el-tooltip class="box-item" effect="dark" placement="top-start" content="自重启以来该任务执行的次数">
                <span>次数<el-icon><InfoFilled /></el-icon></span>
              </el-tooltip>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="100px">
              <template #default="scoped">
                <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row.uuid)"/>
              </template>
          </el-table-column>
        </el-table>
        <div class="pager-container">
          <el-pagination :current-page="task_paging.current"
                         :page-size="task_paging.size" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="task_paging.total" 
                         @size-change="handleTasksSizeChange"
                         @current-change="handleTasksCurrentChange"/>
        </div>
      </el-tab-pane>
      <el-tab-pane label="执行历史" name="history">
        <div class="form">
          <span class="item">
            查询任务<el-dropdown trigger="click">
    <span class="el-dropdown-link">
      <el-tag v-if="uuid_value" type="primary">{{ uuid_value }}</el-tag>
      <el-tag v-else type="primary">所有任务</el-tag>
      <el-icon class="el-icon--right"><caret-bottom /></el-icon>
    </span>
    <template #dropdown>
      <el-dropdown-menu @click="onClearTasks">
        <el-dropdown-item class="clearfix">
          清除当前任务，显示所有任务的历史
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>的执行历史，显示状态为
            <el-radio-group v-model="history_sched_state" @change="fetch_history_states">
              <el-radio-button label="ALL" value="ALL" />
              <el-radio-button label="SUCCESS" value="SUCCESS" />
              <el-radio-button label="FAILED" value="FAILED" />
            </el-radio-group>
            执行时间范围
            <span style="width: 410px">
            <el-date-picker
              v-model="history_execute_times"
              type="datetimerange"
              start-placeholder="开始时间"
              end-placeholder="结束时间"
              format="YYYY-MM-DD HH:mm:ss"
              date-format="YYYY/MM/DD ddd"
              time-format="A hh:mm:ss"
              style="width: 340px;"
            />的记录</span>
            <el-button @click="fetch_history_states" type="primary" style="margin-left: 20px;">查询</el-button>
          </span>
        </div>
        <el-table :data="history_states" ref="redistable" :expand-row-keys="expandedkeys" row-key="id" @expand-change="onTableRowExpanded">
          <el-table-column type="expand">
              <template #default="scoped">
                  <span>执行参数</span>
                  <JsonViewer :value="scoped.row.params" :expand-depth="5" copyable boxed sort></JsonViewer>
              </template>
          </el-table-column>          
          <el-table-column prop="uuid" label="UUID" width="240px" show-overflow-tooltip />
          <el-table-column prop="invoke_uri" label="任务URI"  width="240px" show-overflow-tooltip />
          <el-table-column prop="states" label="状态" />
          <el-table-column prop="sched_type" label="类型">
            <template #default="scoped">
              {{ scoped.row.sched_type === 0 ? '一次性':  scoped.row.sched_type === 1 ? '重复性': 'CRON' }}
            </template>
          </el-table-column>
          <el-table-column prop="sched_express" label="定时器" />
          <el-table-column prop="description" label="任务描述" width="240px" show-overflow-tooltip />
          <el-table-column prop="execute_time" label="上次执行时间" width="160px">
            <template #default="scoped">
              {{ scoped.row.execute_time ? new Date(scoped.row.execute_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
            </template>
          </el-table-column>
          <el-table-column prop="elapsed" label="耗时(微秒)" />
          <el-table-column v-if="false" label="操作" width="100px">
              <template #default="scoped">
                <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row.index)"/>
              </template>
          </el-table-column>
        </el-table>
        <div class="pager-container">
          <el-pagination :current-page="history_paging.current"
                         :page-size="history_paging.size" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="history_paging.total" 
                         @size-change="handleHistorySizeChange"
                         @current-change="handleHistoryCurrentChange"/>
        </div>
      </el-tab-pane>
      <el-tab-pane label="一次性任务" name="oneshot">
        <div class="form">
          <span class="item">
            查询一次性任务的执行历史，显示状态为
            <el-radio-group v-model="oneshot_sched_state" @change="fetch_oneshot_states">
              <el-radio-button label="ALL" value="ALL" />
              <el-radio-button label="SUCCESS" value="SUCCESS" />
              <el-radio-button label="FAILED" value="FAILED" />
            </el-radio-group>
            的记录，
            执行时间范围
            <span style="width: 360px">
            <el-date-picker
              v-model="oneshot_execute_times"
              type="datetimerange"
              start-placeholder="开始时间"
              end-placeholder="结束时间"
              format="YYYY-MM-DD HH:mm:ss"
              date-format="YYYY/MM/DD ddd"
              time-format="A hh:mm:ss"
              style="width: 340px;"
            /></span>
            <el-button @click="fetch_oneshot_states" type="primary" style="margin-left: 20px;">查询</el-button>
          </span>
        </div>
        <el-table :data="oneshot_states" ref="redistable" :expand-row-keys="expandedkeys" row-key="id" @expand-change="onOneshotTableRowExpanded">
          <el-table-column type="expand" class-name="expend-column">
              <template #default="scoped">
                <el-card style="max-width: 100%" shadow="always">
                  <span>执行参数</span>
                  <JsonViewer :value="scoped.row.params" :expand-depth="0" copyable boxed sort></JsonViewer>
                  <el-table :data="oneshot_parent_states" ref="redistable" :expand-row-keys="expandedkeys" header-row-class-name="oneshot_parent" row-class-name="oneshot_parent" row-key="id" @expand-change="onTableRowExpanded">
                    <el-table-column type="expand">
                        <template #default="scoped">
                            <span>执行参数</span>
                            <JsonViewer :value="scoped.row.params" :expand-depth="5" copyable boxed sort></JsonViewer>
                        </template>
                    </el-table-column>
                    <el-table-column prop="uuid" label="子任务UUID" width="240px" show-overflow-tooltip />
                    <el-table-column prop="invoke_uri" label="任务URI"  width="240px" show-overflow-tooltip />
                    <el-table-column prop="states" label="状态" />
                    <el-table-column prop="sched_type" label="类型">
                      <template #default="scoped">
                        {{ scoped.row.sched_type === 0 ? '一次性':  scoped.row.sched_type === 1 ? '重复性': 'CRON' }}
                      </template>
                    </el-table-column>
                    <el-table-column prop="sched_express" label="定时器" />
                    <el-table-column prop="description" label="任务描述" width="240px" show-overflow-tooltip />
                    <el-table-column prop="execute_time" label="上次执行时间" width="160px">
                      <template #default="scoped">
                        {{ scoped.row.execute_time ? new Date(scoped.row.execute_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                      </template>            
                    </el-table-column>
                    <el-table-column prop="elapsed" label="耗时(微秒)" />
                  </el-table>
                </el-card>
              </template>
          </el-table-column>          
          <el-table-column prop="uuid" label="UUID" width="240px" show-overflow-tooltip />
          <el-table-column prop="invoke_uri" label="任务URI"  width="240px" show-overflow-tooltip />
          <el-table-column prop="states" label="状态" />
          <el-table-column prop="sched_type" label="类型">
            <template #default="scoped">
              {{ scoped.row.sched_type === 0 ? '一次性':  scoped.row.sched_type === 1 ? '重复性': 'CRON' }}
            </template>
          </el-table-column>
          <el-table-column prop="sched_express" label="定时器" />
          <el-table-column prop="description" label="任务描述" width="240px" show-overflow-tooltip />
          <el-table-column prop="execute_time" label="上次执行时间" width="160px">
            <template #default="scoped">
              {{ scoped.row.execute_time ? new Date(scoped.row.execute_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
            </template>            
          </el-table-column>
          <el-table-column prop="elapsed" label="耗时(微秒)" />
          <el-table-column prop="times" label="次数" />
          <el-table-column v-if="false" label="操作" width="100px">
              <template #default="scoped">
                <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row.index)"/>
              </template>
          </el-table-column>
        </el-table>
        <div class="pager-container">
          <el-pagination :current-page="oneshot_paging.current"
                         :page-size="oneshot_paging.size" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="oneshot_paging.total" 
                         @size-change="handleOneshotSizeChange"
                         @current-change="handleOneshotCurrentChange"/>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script lang="ts" setup name="redis">
import { computed, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import JsonViewer from 'vue-json-viewer'
import { performance_tasks, performance_oneshots } from "@/http/modules/performance";
import { ElMessage } from "element-plus";
import '@/utils/datetime.js'

const default_page_size = 50
const route = useRoute()
const rediskeys = ref<Array<any>>([])
const task_states = ref<Array<any>>([])
const oneshot_states = ref<Array<any>>([])
const oneshot_parent_states = ref<Array<any>>([])  
  
const select_plugin = ref<string>("")
const task_paging = ref<any>({
  size: default_page_size,
  current: 1,
  total: 0
})
const history_paging = ref<any>({
  size: default_page_size,
  current: 1,
  total: 0
})
const history_states = ref<Array<any>>([])

const oneshot_paging = ref<any>({
  size: default_page_size,
  current: 1,
  total: 0
})

const expandedkeys = ref<Array<any>>([])
const hook = ref<any>({})
const updateIndexMode = ref<boolean>(false)
const showEditForm = ref<boolean>(false)
const activeName = ref("repeatable");
const uuid_value = ref("")
const history_sched_state = ref("")
const oneshot_sched_state = ref("")
const oneshot_execute_times = ref([])
const history_execute_times = ref([])

const auto_refresh = ref(false)
const refresh_seconds = ref(5);

function onClearTasks() {
  uuid_value.value = ''
  fetch_history_states()
}

function onTabPanelClick(t) {
  if (t.props.name === 'history') {
    fetch_history_states()
  } else if (t.props.name === 'oneshot') {
    fetch_oneshot_states()
  } else {
    fetch_task_states()
  }
}

function fetch_task_states() {
  var ns = route.query.ns as string    
  performance_tasks({oneshot: true}).then(res => {
    if (res.data) {
      var tp = task_paging.value;
      task_states.value = res.data.records
      task_paging.value = {
        size: tp.size,
        current: res.data.page_no,
        total: res.data.total
      }
    }
  }).catch(ex => {
    console.log(ex)
  })
}

function fetch_oneshot_states() {
  var ns = route.query.ns as string
  let hpg = oneshot_paging.value
  let het = oneshot_execute_times.value
  let st = oneshot_sched_state.value
  let dt = {
    oneshot: false,
    page_size: hpg.size,
    current: hpg.current,
    sched_type: 0,
    states: st === 'ALL' || st === '' ? null : st,
    execute_begin: null,
    execute_end: null,
  }

  if (het && het.length === 2) {
    dt.execute_begin  = het[0]
    dt.execute_end = het[1]
  }

  performance_oneshots(dt).then(res => {
    if (res.data) {
      var tpp = res.data
      oneshot_states.value = tpp.records
      oneshot_paging.value = {
        current: tpp.page_no,
        size: tpp.page_size,
        total: tpp.total
      }
    }
  }).catch(ex => {
    console.log(ex)
  })
}

function fetch_oneshot_states_by_parent(puuid) {
  let dt = {
    oneshot: false,
    page_size: 1000,
    parent_uuid: puuid,
    current: 1,
    states: null,
    execute_begin: null,
    execute_end: null,
  }

  performance_oneshots(dt).then(res => {
    if (res.data) {
      var tpp = res.data
      oneshot_parent_states.value = tpp.records
    }
  }).catch(ex => {
    console.log(ex)
  })
}

function fetch_history_states() {
  var ns = route.query.ns as string
  let hpg = history_paging.value
  let st = history_sched_state.value
  let het = history_execute_times.value
  let uuid = uuid_value.value
  let dt = {
    oneshot: false,
    uuid: (uuid && uuid !== '') ? uuid : null,
    sched_type: 1,
    states: st === 'ALL' || st === '' ? null : st,
    page_size: hpg.size,
    current: hpg.current,
    execute_begin: null,
    execute_end: null,
  }

  if (het && het.length === 2) {
    dt.execute_begin  = het[0]
    dt.execute_end = het[1]
  }

  performance_tasks(dt).then(res => {
    if (res.data) {
      var tpp = res.data
      history_states.value = tpp.records
      history_paging.value = {
        current: tpp.page_no,
        size: tpp.page_size,
        total: tpp.total
      }
    } else {
      history_states.value = []
    }
  }).catch(ex => {
    console.log(ex)
  })
}

function onEditKey(row) {
  uuid_value.value = row
  activeName.value = "history"
  fetch_history_states()
}


function onTableRowExpanded(row: any, expanded: any[]) {
  console.log('ex', expanded)
  if (expanded.length > 0) {
    var ns = route.query.ns as string
    var method = "setting:" + row.index
  }
}

function onOneshotTableRowExpanded(row: any, expanded: any[]) {
  if (expanded.length > 0) {
    fetch_oneshot_states_by_parent(row.uuid)
  }
}

function handleTasksSizeChange(e) {
  console.log(e)
  const pg = task_paging.value
  task_paging.value = {...pg, current: 1, size: e}
}

function handleTasksCurrentChange(e) {
  console.log('current', e)
  const pg = task_paging.value
  task_paging.value = {...pg, current: e}
}

function handleHistorySizeChange(e) {
  const pg = history_paging.value
  history_paging.value = {...pg, current: 1, size: e}
  fetch_history_states()
}

function handleHistoryCurrentChange(e) {
  console.log('current', e)
  const pg = history_paging.value
  history_paging.value = {...pg, current: e}
  fetch_history_states()
}

function handleOneshotSizeChange(e) {
  console.log(e)
  const pg = oneshot_paging.value
  oneshot_paging.value = {...pg, current: 1, size: e}
  fetch_oneshot_states()
}

function handleOneshotCurrentChange(e) {
  const pg = oneshot_paging.value
  oneshot_paging.value = {...pg, current: e}
  fetch_oneshot_states()
}

const tasksList = computed(() => {
  const pg = task_paging.value
  const start = (pg.current - 1) * pg.size;
  const end = start + pg.size;
  const et = task_states.value.slice(start, end);
  console.log(start, end, et)
  return task_states.value.slice(start, end);
})

const timer_interval = ref(null)

function startFetchInterval() {
  let rs = refresh_seconds.value
  if (timer_interval.value) {
    clearInterval(timer_interval.value)
  } 

  if (auto_refresh.value) {
    timer_interval.value = setInterval(() => {
      fetch_task_states()
    }, rs * 1000)
  } else {
    fetch_task_states()
  }
}

onMounted(() => {
  fetch_task_states()
});
</script>

<style lang="scss" scoped>
@use "index.scss";
.title {
  height: 40px;
  font-size: 24px;
}
.form {
  display: block;
  height: 40px;
  border-bottom: 1px solid #ccc;
}
.form .item {
  display: flex;
  flex-direction: row;
  justify-content: flex-start;
  align-items: center;
  height: 40px;
  vertical-align: middle;
}
.pager-container {
  padding: 10px 0px 10px 0px;
}


.oneshot_parent {
  background-color: aqua;
}
</style>
