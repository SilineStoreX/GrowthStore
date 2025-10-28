<template>
    <div class="home">
      <div>
        <el-form label-width="100px" :inline="true">
          <el-form-item label="类型">
            <el-checkbox-group v-model="cmdfrm.selected_types" size="default">
              <el-checkbox-button v-for="t in prompt_types" :key="t" :value="t">
                {{ t }}
              </el-checkbox-button>
            </el-checkbox-group>
          </el-form-item>
          <el-form-item label="角色">
            <el-checkbox-group v-model="cmdfrm.selected_roles" size="default">
              <el-checkbox-button v-for="t in prompt_roles" :key="t" :value="t">
                {{ t }}
              </el-checkbox-button>
            </el-checkbox-group>
          </el-form-item>
          <el-form-item label="名称">
              <el-input v-model="cmdfrm.name"></el-input>
          </el-form-item>
          <el-form-item>
              <el-button type="primary" @click="onQueryKeys">查询</el-button>
              <el-button type="success" @click="onCreateIndex">创建</el-button>
          </el-form-item>
          <el-form-item label="详细描述">
              <el-input v-model="cmdfrm.description" type="textarea" style="width: 620px; margin-right: 10px;"></el-input>
              <el-button @click="onQueryTest">召回测试</el-button>
          </el-form-item>
          <el-form-item label="召回参数" style="padding-top: -10px;">
              <el-form-item label="相似度大于"><el-input v-model="cmdfrm.score_threshold" /></el-form-item>
              <el-form-item label="Top-K"><el-input v-model="cmdfrm.top_k" /></el-form-item>
          </el-form-item>          
        </el-form>
        <el-divider />
      </div>
      <el-table :data="rediskeys" v-loading="showLoading" ref="redistable" :height="scrollHeight - 190">
        <el-table-column prop="id" label="ID" width="60px" />
        <el-table-column prop="name" label="名称" width="180px" />
        <el-table-column prop="type" label="类型" width="100px" />
        <el-table-column prop="intent_id" label="意图" width="100px">
          <template #default="scoped">
            {{ get_intent(scoped.row.intent_id) }}
          </template>
        </el-table-column>
        <el-table-column prop="role" label="角色" width="100px" />
        <el-table-column prop="default_prompt" label="缺省" width="80px">
          <template #default="scoped">
            {{ scoped.row.default_prompt ? '是': '否' }}
          </template>
        </el-table-column>
        <el-table-column prop="description" label="意图描述" show-overflow-tooltip />
        <el-table-column v-if="showSimilarity" prop="similarity" label="相似度" width="120px">
          <template #default="scoped">
            {{ scoped.row.similarity ? scoped.row.similarity.toFixed(5): '' }}
          </template>          
        </el-table-column>
        <el-table-column v-if="!showSimilarity" label="状态" width="80px">
          <template #default="scoped">
            <el-tag v-if="scoped.row.status === 0" type="success">成功</el-tag>
            <el-tag v-else-if="scoped.row.status === 1" type="primary">等待</el-tag>
            <el-tag v-else-if="scoped.row.status === 2" type="warning">处理中</el-tag>
            <el-tag v-else-if="scoped.row.status === 5" type="danger">失败</el-tag>
            <el-tag v-else type="info">未处理</el-tag>
          </template>
        </el-table-column>
        <el-table-column v-if="!showSimilarity" prop="update_time" label="更新时间" width="220px">
          <template #default="scoped">
            {{ scoped.row.update_time ? new Date(scoped.row.update_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
          </template>
        </el-table-column>
        <el-table-column v-if="!showSimilarity" label="操作" width="100px">
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
          <el-pagination v-if="!showSimilarity" :current-page="PageCurrent"
                         :page-size="PageSize" 
                         background layout="total, sizes, prev, pager, next"
                         :page-sizes="[20, 50, 100, 200, 300, 400]" :total="PageTotal" 
                         @size-change="handleSizeChange"
                         @current-change="handleCurrentChange"/>
      </div>
      <el-drawer title="编辑提示词" v-model="showEditForm" :with-header="true" :size="900">
        <el-form v-model="editfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="类型">
            <el-radio-group v-model="editfrm.type" size="default" @change="fetch_parent_prompts">
              <el-radio-button v-for="t in prompt_types" :key="t" :value="t">
                {{ t }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="角色">
            <el-radio-group v-model="editfrm.role" size="default">
              <el-radio-button v-for="t in prompt_roles" :key="t" :value="t">
                {{ t }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item v-if="editfrm.type === 'rag'" label="缺省提示词">
            <el-switch v-model="editfrm.default_prompt" style="width: 280px" />
            每个类型可以创建一个缺省提示词，如果实际上存在多个缺省提示词，则以最早创建的缺省提示词为准。
          </el-form-item>
          <el-form-item v-if="editfrm.type === 'executing' || editfrm.type === 'summary' || editfrm.type === 'classification'" label="父级提示词">
            <el-select v-model="editfrm.parent_id" clearable style="width: 700px">
              <el-option v-for="it in parent_prompts" :key="it.id" :value="it.id" :label="it.name">{{ it.name }}</el-option>
            </el-select>
          </el-form-item>
          <el-form-item label="意图类型">
            <el-select v-model="editfrm.intent_id" style="width: 700px">
              <el-option v-for="it in all_intents" :key="it.id" :value="it.id" :label="it.intent">{{ it.intent }}</el-option>
            </el-select>
          </el-form-item>
          <el-form-item label="名称">
            <el-input v-model="editfrm.name" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="意图描述">
            <el-input v-model="editfrm.description" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
          <el-form-item label="提示词模板">
            <el-input v-model="editfrm.prompt_template" type="textarea" style="width: 700px" :rows="15" placeholder="请输入提示词的模板，在提示词模板中可以使用下述占位符来引用对应的内容。"></el-input>
          </el-form-item>
          <el-form-item label="查询占位符">
            <el-input v-model="editfrm.query_holder" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="上下文占位符">
            <el-input v-model="editfrm.context_holder" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="历史占位符">
            <el-input v-model="editfrm.history_holder" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="处理状态">
            <el-tag v-if="editfrm.status === 0" type="success">成功</el-tag>
            <el-tag v-else-if="editfrm.status === 1" type="primary">等待</el-tag>
            <el-tag v-else-if="editfrm.status === 2" type="warning">处理中</el-tag>
            <el-tag v-else-if="editfrm.status === 5" type="danger">失败</el-tag>
            <el-tag v-else type="info">未处理</el-tag>
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
  
  <script lang="ts" setup name="prompts">
  import { onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import '@/utils/datetime.js'
  import { prompts_search, prompts_save, prompts_delete, prompts_get, prompts_retrieval_test, intent_query } from "@/http/modules/growthai";
  import { ElMessage } from "element-plus";


  const route = useRoute()
  const prompt_types = ref<Array<any>>(["planning", "rag", "executing", "summary"])
  const prompt_roles = ref<Array<any>>(["system", "user", "assistance"])
  const PageCurrent = ref<any>(1)
  const PageSize = ref<any>(20)
  const PageTotal = ref(0)
  const scrollHeight = ref<any>(600)
  const parent_prompts = ref<Array<any>>([])
  const rediskeys = ref<Array<any>>([])
  const all_intents = ref<Array<any>>([])
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

  function onCreateIndex() {
    editfrm.value = {}
    showEditForm.value = true
  }

  function onDialogClosed() {
    showEditForm.value = false
  }

  function onEditKey(row) {
    var ns = route.query.ns as string
    editfrm.value = row
    showEditForm.value = true
    showLoading.value = true
    prompts_get(ns, row.id).then(res => {
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
    data.status = 1
    showLoading.value = true
    prompts_save(ns, data).then(res => {
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
    prompts_delete(ns, { id: key }).then(res => {
      fetch_indexes()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetch_intents() {
    var ns = route.query.ns as string
    intent_query(ns, {}).then(res => {
      if (res.status === 0 || res.status === 200) {
        all_intents.value = res.data
      }
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
      paging: {
        size: PageSize.value,
        current: PageCurrent.value
      }
    };

    if (queryfrm.selected_types && queryfrm.selected_types.length > 0) {
      cond.and.push({field: 'type', op: 'IN', value: queryfrm.selected_types})
    }

    if (queryfrm.selected_roles && queryfrm.selected_roles.length > 0) {
      cond.and.push({field: 'role', op: 'IN', value: queryfrm.selected_roles})
    }

    if (queryfrm.name && queryfrm.name !== '') {
      cond.and.push({field: 'name', op: 'LIKE', value: queryfrm.name + '%'})
    }
    showLoading.value = true
    prompts_search(ns, cond).then(res => {
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

  function fetch_parent_prompts(e) {
    var ns = route.query.ns as string
    var queryfrm = editfrm.value
    var cond = {
      and: [
      ],
      paging: {
        size: 1000,
        current: 1
      }
    };

    if (e !== 'rag') {
      editfrm.value.default_prompt = false
    } 
    
    if (e === 'planning' || e === 'rag'){
      editfrm.value.parent_id = null
    }

    if (queryfrm.type === 'executing' || queryfrm.type === 'summary') {
      cond.and.push({field: 'type', op: 'IN', value: ["planning", "executing", "rag"]})
    }

    showLoading.value = true
    prompts_search(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        PageTotal.value = res.data.total
        parent_prompts.value = res.data.records
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function do_prompts_retrieval_test() {
    var ns = route.query.ns as string
    var queryfrm = cmdfrm.value
    var data = {
      query: queryfrm.description,
      retrieval_setting: {
        top_k: queryfrm.top_k,
        score_threshold: queryfrm.score_threshold
      }
    }

    showLoading.value = true
    prompts_retrieval_test(ns, data).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        PageTotal.value = 0
        rediskeys.value = res.records
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
    do_prompts_retrieval_test()
  }

  function onResize() {
    scrollHeight.value = window.innerHeight - 240;
  }

  function get_intent(it) {
    let data = all_intents.value
    let t = data.find(p => p.id === it);
    if (t) {
      return t.intent
    } else {
      return "--通用--"
    }
  }

  onMounted(() => {
    fetch_indexes()
    fetch_intents()
    onResize()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  