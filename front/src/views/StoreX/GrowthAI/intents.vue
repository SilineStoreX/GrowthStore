<template>
    <div class="home">
      <el-row :gutter="20">
        <el-col :span="6">
          <div class="kbheader">
            <span class="text-large font-600 mr-3">意图类型<el-button type="primary" icon="Plus" text @click="onAddIntent">新建</el-button></span>
            <el-divider />
          </div>
          <el-scrollbar :height="scrollHeight">
            <el-card v-for="kb in intents" :key="kb.id" shadow="hover" :class="'kbcard ' + (current_intent.id === kb.id ? 'selected' : '')">
              <div class="kb-container" @click="onSelectIntent(kb)">
                <el-icon><Present /></el-icon> <span>{{ kb.intent }}</span>
              </div>
              <div class="kb-action-right">
                <div class="actions">
                  <el-button icon="Edit" type="primary" text @click="onEditIntent(kb)">编辑</el-button>
                  <el-popconfirm title="确定要删除该意图类型吗?" @confirm="onDelIntent(kb)">
                    <template #reference>
                        <el-button icon="Delete" type="danger" text>删除</el-button>
                    </template>
                  </el-popconfirm>
                </div>
              </div>
            </el-card>
            <el-card class="kbcard" shadow="hover">
              <div class="plus-container" @click="onAddIntent">
                <el-icon><Plus /></el-icon>
              </div>
            </el-card>
          </el-scrollbar>
        </el-col>
        <el-col :span="18">
          <el-header v-if="current_intent && current_intent.id" class="intent_page_header">
            <div class="topbar">
              <div class="title">{{ current_intent.intent }}</div>
              <div class="action">
                <el-button type="success" icon="Plus" text @click="onAddIntentExample">新建训练数据</el-button>
              </div>
            </div>
            <el-divider />
            <div class="topbar">
              <div class="action">
                <el-form v-model="queryfrm" :inline="true">
                  <el-form-item label="来源">
                    <el-radio-group v-model="queryfrm.training_source" size="default" @change="onIntentExampleQuery">
                      <el-radio-button value="All">All</el-radio-button>
                      <el-radio-button value="Bootstrap">Bootstrap</el-radio-button>
                      <el-radio-button value="UserFeedback">UserFeedback</el-radio-button>
                      <el-radio-button value="Programmatic">Programmatic</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="标签">
                    <el-input v-model="queryfrm.tags" />
                  </el-form-item>
                </el-form>
              </div>
              <div class="action">
                <el-form-item>
                  <el-button type="success" icon="Search" @click="onIntentExampleQuery">查询</el-button>
                </el-form-item>
              </div>
            </div>
          </el-header>
            <el-table :data="intent_examples" v-loading="showLoading" ref="redistable" :height="scrollHeight - 120">
              <el-table-column type="index" width="60" label="序号" />
              <el-table-column prop="training_source" label="源" width="100px" />
              <el-table-column prop="training_text" label="训练文本" show-overflow-tooltip />
              <el-table-column prop="confidence" label="相似度" width="100px" />
              <el-table-column prop="tags" label="标签" width="120px" show-overflow-tooltip />
              <el-table-column prop="update_time" label="更新时间" width="200px">
                <template #default="scoped">
                  {{ scoped.row.update_time ? new Date(scoped.row.update_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                </template>
              </el-table-column>
              <el-table-column label="操作" width="100px">
                  <template #default="scoped">
                    <el-button type="primary" icon="SetUp" circle @click="onEditChunk(scoped.row)"/>
                    <el-popconfirm title="确认要删除吗?" @confirm="onDelChunk(scoped.row)">
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
        </el-col>
      </el-row>
      <el-drawer title="编辑意图类型" v-model="showEditForm" :with-header="true" :size="900">
        <el-form v-model="docfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="意图类型">
            <el-input v-model="docfrm.intent" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="详细描述">
            <el-input v-model="docfrm.description" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该意图"></el-input>
          </el-form-item>
          <el-form-item label="信念度">
            <el-input v-model="docfrm.confidence" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="路由大模型">
            <el-input v-model="docfrm.route_model" style="width: 700px"></el-input>
            <el-text>对于该意图，将路由到指定的大模型进行处理。</el-text>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="primary" @click="onConfirmDocument">
            <el-icon class="el-icon--left"><CircleCheckFilled /></el-icon>
            保存
          </el-button>
          <el-button type="danger" @click="onDialogClosed">
            <el-icon class="el-icon--left"><CircleCloseFilled /></el-icon>
            关闭
          </el-button>
        </template>
      </el-drawer>
      <el-drawer title="编辑训练数据" v-model="showChunkEditForm" :with-header="true" :size="900">
        <el-form v-model="chunkfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="来源">
            <el-radio-group v-model="chunkfrm.training_source" size="default">
              <el-radio-button value="Bootstrap">Bootstrap</el-radio-button>
              <el-radio-button value="UserFeedback">UserFeedback</el-radio-button>
              <el-radio-button value="Programmatic">Programmatic</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="文本块">
            <el-input v-model="chunkfrm.training_text" type="textarea" style="width: 700px" :rows="6" placeholder="请填写训练文本"></el-input>
          </el-form-item>
          <el-form-item label="信念度">
            <el-input v-model="chunkfrm.confidence" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="标签">
            <el-input v-model="chunkfrm.tags" style="width: 700px" placeholder="请以半角逗号分隔"></el-input>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="primary" @click="onConfirmChunk">
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
  import { computed, onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import '@/utils/datetime.js'
  import { intent_query, intent_save, intent_delete } from "@/http/modules/growthai";
  import { intent_example_query, intent_example_save, intent_example_delete } from "@/http/modules/growthai";
  import { ElMessage } from "element-plus";
  import { GlobalStore } from "@/stores";

  const globalStore = GlobalStore();
  const route = useRoute()
  const queryfrm = ref<any>({})
  const PageCurrent = ref<any>(1)
  const PageSize = ref<any>(20)
  const PageTotal = ref(0)
  
  const current_intent = ref<any>({})
  const scrollHeight = ref<any>(600)
  const intents = ref<Array<any>>([])
  const intent_examples = ref<Array<any>>([])

  const docfrm = ref<any>({})
  const chunkfrm = ref<any>({})

  const showChunkEditForm = ref(false)
  const showDocumentEditForm = ref(false)
  const showStructEditForm = ref(false)

  const showSimilarity = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)
  const showLoading = ref<boolean>(false)
  
  function get_auth_header() {
    return {
      "Authorization": "Bearer " + globalStore.token
    }
  }

  const upload_file_uri = computed(() => {
    var ns = route.query.ns as string
    return `/api/compose/${ns}/compose/uploadavatar/upload`
  })

  function handleSizeChange(e) {
    PageSize.value = e
    fetch_chunksets()
  }

  function handleCurrentChange(e) {
    PageCurrent.value = e
    fetch_chunksets()
  }

  function onAddIntentExample() {
    if (current_intent.value && current_intent.value.id) {
      chunkfrm.value = {
        intent_id: current_intent.value.id,
      }
      showChunkEditForm.value = true
    } else {
      ElMessage.info("请先选择意图类型！")
    }
  }

  function onAddIntent() {
    docfrm.value = {}
    showEditForm.value = true
  }

  function onEditIntent(kb) {
    docfrm.value = kb
    showEditForm.value = true    
  }

  function onEditChunk(row) {
    chunkfrm.value = row
    showChunkEditForm.value = true
  }

  function onDialogClosed() {
    showEditForm.value = false
    showChunkEditForm.value = false
    showDocumentEditForm.value = false
    showStructEditForm.value = false
  }

  function onConfirmChunk() {
    var ns = route.query.ns as string
    var data = chunkfrm.value
    // data.chunk_status = "PENDING"; // pending to index
    showLoading.value = true
    intent_example_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_chunksets()
        showChunkEditForm.value = false
      } else {
        ElMessage.warning("保存失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("保存失败：" + ex.message)
    })
  }

  function onConfirmDocument() {
    var ns = route.query.ns as string
    var data = docfrm.value
    data.status = 1; // pending to index
    showLoading.value = true
    intent_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_intents()
        showEditForm.value = false
      } else {
        ElMessage.warning("保存失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("保存失败：" + ex.message)
    })
  }

  function onDelChunk(row) {
    var ns = route.query.ns as string
    intent_example_delete(ns, { id: row.id }).then(res => {
      fetch_chunksets()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onDelIntent(row) {
    var ns = route.query.ns as string
    intent_delete(ns, { id: row.id }).then(_ => {
      fetch_intents()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onSelectIntent(row) {
    var ns = route.query.ns as string
    current_intent.value = row
    fetch_chunksets()
  }

  function onIntentExampleQuery() {
    PageCurrent.value = 1
    fetch_chunksets()
  }

  function fetch_intents() {
    var ns = route.query.ns as string
    var cond = {}
    showLoading.value = true
    intent_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        intents.value = res.data
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function fetch_chunksets() {
    var ns = route.query.ns as string
    var query = queryfrm.value
    var cond = {
      and: [{field: 'intent_id', op: '=', value: current_intent.value.id }],
      paging: {
        current: PageCurrent.value,
        size: PageSize.value
      }
    }

    if (query.tags && query.tags.length > 0) {
      cond.and.push({field: "tags", op: 'LIKE', value: '%' + query.tags + '%'})
    }

    if (query.training_source && query.training_source !== '' && query.training_source !== 'All') {
      cond.and.push({field: "training_source", op: '=', value: query.training_source })
    }

    showSimilarity.value = false

    showLoading.value = true
    intent_example_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        intent_examples.value = res.data.records
        PageTotal.value = res.data.total
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function onResize() {
    scrollHeight.value = window.innerHeight - 240;
  }

  onMounted(() => {
    fetch_intents()
    onResize()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  