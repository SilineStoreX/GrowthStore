<template>
  <el-dialog
      v-model="props.show"
      title="管理变量"
      width="760"
      align-center
      @close="onDialogClosed"
    >
    <el-table :data="variables" :height="460">
        <el-table-column prop="variable_name" label="变量名称" width="160px">
          <template #default="scoped">
            <el-input v-if="scoped.row._editing" v-model="scoped.row.variable_name" />
            <el-text v-else>{{ scoped.row.variable_name }}</el-text>
          </template>
        </el-table-column>
        <el-table-column prop="variable_type" label="类型"  width="180px">
          <template #default="scoped">
            <el-radio-group v-if="scoped.row._editing" v-model="scoped.row.variable_type" size="small">
              <el-radio-button value="Number">Number</el-radio-button>
              <el-radio-button value="DateTime">DateTime</el-radio-button>
            </el-radio-group>
            <el-text v-else>{{ scoped.row.variable_type }}</el-text>
          </template>
        </el-table-column>
        <el-table-column prop="last_value" label="当前值">
          <template #default="scoped">
            <el-input v-if="scoped.row._editing" v-model="scoped.row.last_value" />
            <el-text v-else>{{ scoped.row.last_value }}</el-text>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150px">
            <template #default="scoped">
                <el-button v-if="scoped.row._editing" type="primary" icon="Check" @click="handleSaveVariable(scoped.row)">保存</el-button>
                <el-tooltip v-if="scoped.row._editing" class="box-item" effect="dark" placement="top-start" content="取消编辑">
                  <el-button type="danger" icon="RefreshLeft" circle @click="handleCancelEditVariable(scoped.row)" />
                </el-tooltip>
                <el-button v-if="!scoped.row._editing" type="primary" icon="Edit" circle @click="handleModifyVariable(scoped.row)" />
                <el-popconfirm v-if="!scoped.row._editing" title="确认要删除吗?" @confirm="handleRemoveVariable(scoped.row)">
                    <template #reference>
                        <el-button type="danger" icon="Delete" circle />
                    </template>
                </el-popconfirm>
            </template>
        </el-table-column>
        <template #append>
          <div style="padding: 10px; display: flex; justify-content: center;">
            <el-button icon="Plus" @click="handleAddVariable">添加变量</el-button>
          </div>
        </template>
    </el-table>
    <template #footer>
      <div class="dialog-footer">
        <el-button type="primary" @click="onDialogClosed">关闭</el-button>
      </div>
    </template>
  </el-dialog>
</template>
<script lang="ts" setup name="Variables">
import { onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import '@/utils/datetime.js'
import { variable_query, variable_delete, variable_save } from "@/http/modules/growthai";

const emit = defineEmits(['update:show'])
const props = defineProps<{ show: boolean }>();
const variables = ref<Array<any>>([])
const route = useRoute()
const showLoading = ref(false)

watch(
  () => props.show,
  () => {
    if (props.show) {
      fetch_variables()
    }
  },
  {
    immediate: true,
  }
);

function onDialogClosed() {
  emit('update:show', false)
}

function handleModifyVariable(row) {
  row._editing = true
}

function handleAddVariable() {
  let v = variables.value;
  if (v.find(r => r._editing === true)) {
    return
  } else {
    v.map(r => {
      r._editing = false
      r
    })

    v.push({
      _editing: true
    })
    variables.value = v
  }
}

function handleCancelEditVariable(row) {
  row._editing = false
}

function handleSaveVariable(row) {
  var ns = route.query.ns as string
  var cond = {
    ...row
  }
  showLoading.value = true
  variable_save(ns, cond).then(res => {
    showLoading.value = false
    fetch_variables()
  }).catch(ex => {
    showLoading.value = false
  })
}

function handleRemoveVariable(row) {
  var ns = route.query.ns as string
  var cond = {
    id: row.id
  }
  showLoading.value = true
  variable_delete(ns, cond).then(res => {
    showLoading.value = false
    fetch_variables()
  }).catch(ex => {
    showLoading.value = false
  })
}

function fetch_variables() {
  var ns = route.query.ns as string
  var cond = {}
  showLoading.value = true
  variable_query(ns, cond).then(res => {
    showLoading.value = false
    if(res.data) {
      variables.value = res.data
    }
  }).catch(ex => {
    showLoading.value = false
    console.log(ex)
  })
}

onMounted(() => {
});
</script>

<style lang="scss" scoped>
@use "index.scss";
</style>
  