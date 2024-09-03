<template>
  <el-dialog
    v-model="props.visible"
    title="添加存储对象服务"
    width="960"
    align-center
    @close="onDialogClosed"
  >
    <el-form label-width="100px" :inline="true">
        <el-form-item label="Schema">
            <el-input v-model="query.schema_name" />
        </el-form-item>
        <el-form-item label="Table">
            <el-input v-model="query.table_name" />
        </el-form-item>
        <el-form-item label=" ">
            <el-button type="success" @click="handleProbe">查询</el-button>
        </el-form-item>
    </el-form>
    <el-table :data="tables"  max-height="300" @selection-change="handleSelectionChange">
        <el-table-column label="Schema" prop="table_schema" width="120px" />
        <el-table-column label="目录" prop="table_catalog"  width="120px" />
        <el-table-column label="类型" prop="table_type"  width="120px" />
        <el-table-column label="对象名" prop="table_name" width="240px"/>
        <el-table-column label="引用名" prop="name" width="240px">
          <template #default="scoped">
            <el-input v-model="scoped.row.name" />
          </template>
        </el-table-column>
        <el-table-column type="selection" width="55" />
    </el-table>
    <template #footer>
      <div class="dialog-footer">
        <el-form label-width="100px" :inline="true">
          <el-form-item label="属性名规则" style="float: left">
              <el-select v-model="query.rule" style="width: 200px">
                <el-option value="none" label="与字段名相同">与字段名相同</el-option>
                <el-option value="snakecase"  label="Snake Case">Snake Case</el-option>
                <el-option value="camelcase"  label="Camel Case">Camel Case</el-option>
                <el-option value="pascalcase"  label="Pascal Case">Pascal Case</el-option>
                <el-option value="mixup"  label="Mixup">Mixup</el-option>
              </el-select>
          </el-form-item>
          <el-button class="el-form-item" @click="$emit('update:visible', false)">取消</el-button>
          <el-button class="el-form-item" type="primary" @click="onConfirm">
            确认
          </el-button>
        </el-form>
      </div>
    </template>
  </el-dialog>
</template>

<script lang="ts" setup name="config">
import { probeTables, generate } from "@/http/modules/management";
import { useRoute } from "vue-router";
import { ElMessage } from "element-plus";
import { mergeProps, onMounted, ref, watch } from "vue";
const props = defineProps<{ visible: boolean }>();
const emit = defineEmits(['update:visible'])
const tables = ref<Array<any>>([])
const selections = ref<Array<any>>([])
const query = ref<any>({})
const route = useRoute()

function handleProbe() {
  var ns = route.query.ns
  var q = query.value

  probeTables(ns as string, q.schema_name).then(res => {
    if (res.status === 0 || res.status === 200) {
      let answer = res.data.map((dt: any) => {
        dt.name = changePascal(dt.table_name)
        return dt
      })
      tables.value = answer
    } else {
      ElMessage.warning("查询失败：" + res.message)
    }
  }).catch(ex => {
    console.log(ex)
    ElMessage.warning("查询失败：" + ex.message)
  })
} 

function handleSelectionChange(e: any) {
  console.log(e)
  selections.value = e
}

function onDialogClosed() {
  emit('update:visible', false)
}

function changePascal(name) {
    let arr = name.split('_');
    let cp = arr.map((item) => {
      console.log(item)
      if (item.length > 0) {
        var cap = item[0].toUpperCase()
        return cap + item.substring(1)
      } else {
        return item
      }
    })
    console.log(cp.join(''))
    return cp.join('')
}

function onConfirm() {
  var ns = route.query.ns as string
  var q = query.value

  var tbls = selections.value.map(v => {
    return {
      object_name: v.table_name,
      name: v.name,
      fields: [],
      select_sql: '',
    }
  })
  
  console.log('select', tbls)
  generate(ns, q.schema_name, q.rule, tbls).then(res => {
    emit('update:visible', false)
  }).catch(ex => {
    console.log(ex)
  })
}

onMounted(() => {
    console.log("config");
});
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
