<template>
    <div class="home">
      <div>
        <el-form label-width="100px" :inline="true">
          <el-form-item label="角色">
            <el-checkbox-group v-model="cmdfrm.selected_roles" size="default">
              <el-checkbox-button v-for="t in prompt_roles" :key="t" :value="t">
                {{ t }}
              </el-checkbox-button>
            </el-checkbox-group>
          </el-form-item>
          <el-form-item label="用户名">
              <el-input v-model="cmdfrm.username"></el-input>
          </el-form-item>
          <el-form-item label="会话ID">
              <el-input v-model="cmdfrm.session_id"></el-input>
          </el-form-item>
          <el-form-item>
              <el-button type="primary" @click="onQueryKeys">查询</el-button>
          </el-form-item>
        </el-form>
        <el-divider />
      </div>
      <el-table :data="rediskeys" v-loading="showLoading" ref="redistable" :height="scrollHeight - 70">
        <el-table-column prop="id" label="ID" width="60px" />
        <el-table-column prop="username" label="用户名" width="180px" />
        <el-table-column prop="session_id" label="会话" width="100px" show-overflow-tooltip />
        <el-table-column prop="role" label="角色" width="100px" />
        <el-table-column prop="query" label="用户查询" show-overflow-tooltip />
        <el-table-column v-if="showSimilarity" prop="query_similarity" label="查询相似度" width="150px" />
        <el-table-column v-if="showSimilarity" prop="similarity" label="回复相似度" width="150px" />
        <el-table-column prop="update_time" label="更新时间" width="220px">
          <template #default="scoped">
            {{ scoped.row.update_time ? new Date(scoped.row.update_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100px">
            <template #default="scoped">
              <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row)"/>
              <el-popconfirm title="确认要删除吗?" @confirm="onDelKey(scoped.row.id)">
                  <template #reference>
                      <el-button type="danger" icon="Delete" circle />
                  </template>
              </el-popconfirm>
            </template>
        </el-table-column>
      </el-table>
      <div class="pager-container">
          <el-pagination :current-page="PageCurrent"
                         :page-size="PageSize" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="PageTotal" 
                         @size-change="handleSizeChange"
                         @current-change="handleCurrentChange"/>
      </div>
      <el-drawer title="编辑历史会话" v-model="showEditForm" :with-header="true" :size="900">
        <el-form v-model="editfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="用户名">
            <el-input v-model="editfrm.username" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="角色">
            <el-radio-group v-model="editfrm.role" size="default">
              <el-radio-button v-for="t in prompt_roles" :key="t" :value="t">
                {{ t }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="会话">
            <el-input v-model="editfrm.session_id" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="用户提问">
            <el-input v-model="editfrm.query" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
          <el-form-item label="AI回复">
            <el-input v-model="editfrm.content" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="primary" @click="onConfirm">
            <el-icon class="el-icon--left"><CircleCheckFilled /></el-icon>
            保存
          </el-button>
          <el-button type="danger" @click="onDialogClosed">
            <el-icon class="el-icon--left"><CircleCloseFilled /></el-icon>
            关闭
          </el-button>
        </template>
      </el-drawer>
    </div>
  </template>
  
  <script lang="ts" setup name="Memories">
  import { onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import '@/utils/datetime.js'
  import { memory_search, memory_save, memory_delete, memory_get } from "@/http/modules/growthai";
  import { ElMessage } from "element-plus";


  const route = useRoute()
  const prompt_roles = ref<Array<any>>(["system", "user", "assistance"])
  const PageCurrent = ref<any>(1)
  const PageSize = ref<any>(20)
  const PageTotal = ref(0)
  const scrollHeight = ref<any>(600)
  const rediskeys = ref<Array<any>>([])
  const cmdfrm = ref<any>({})
  const editfrm = ref<any>({})
  const hook = ref<any>({})
  const showSimilarity = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)
  const showLoading = ref<boolean>(false)

  function onDelKeys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    onDelKey(key)
  }

  function handleSizeChange(e) {
    PageSize.value = e
    fetch_indexes()
  }

  function handleCurrentChange(e) {
    PageCurrent.value = e
    fetch_indexes()
  }

  function onDialogClosed() {
    showEditForm.value = false
  }

  function onEditKey(row) {
    var ns = route.query.ns as string
    editfrm.value = row
    showEditForm.value = true
    showLoading.value = true
    memory_get(ns, row.id).then(res => {
      showLoading.value = false
      if (res.data) {
        editfrm.value = res.data
      }
    }).catch(ex => {
      showLoading.value = false
      ElMessage.error("获取提示词内容失败。")
    })
  }
  
  function onConfirm() {
    var es = hook.value
    var ns = route.query.ns as string
    var data = editfrm.value
    showLoading.value = true
    memory_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_indexes()
        showEditForm.value = false
      } else {
        ElMessage.warning("执行失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("执行失败：" + ex.message)
    })
  }

  function onDelKey(key) {
    var ns = route.query.ns as string
    memory_delete(ns, { id: key }).then(res => {
      fetch_indexes()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetch_indexes() {
    var ns = route.query.ns as string
    var queryfrm = cmdfrm.value
    var cond = {
      and: [
      ],
      sorts: [{field: "create_time", sort_asc: false}],
      paging: {
        size: PageSize.value,
        current: PageCurrent.value
      }
    };


    if (queryfrm.selected_roles && queryfrm.selected_roles.length > 0) {
      cond.and.push({field: 'role', op: 'IN', value: queryfrm.selected_roles})
    }

    if (queryfrm.username && queryfrm.username !== '') {
      cond.and.push({field: 'username', op: 'LIKE', value: queryfrm.username + '%'})
    }

    if (queryfrm.session_id && queryfrm.session_id !== '') {
      cond.and.push({field: 'session_id', op: '=', value: queryfrm.session_id })
    }

    showLoading.value = true
    memory_search(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        PageTotal.value = res.data.total
        rediskeys.value = res.data.records
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function onQueryKeys() {
    showSimilarity.value = false
    fetch_indexes()
  }

  function onQueryTest() {
    showSimilarity.value = true
    fetch_indexes()
  }

  function onResize() {
    scrollHeight.value = window.innerHeight - 240;
  }

  onMounted(() => {
    fetch_indexes()
    onResize()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  