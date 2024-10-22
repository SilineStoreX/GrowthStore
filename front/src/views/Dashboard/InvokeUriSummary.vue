<template>
    <el-table :data="chartData" ref="redistable" row-key="full_url" :border="true" max-height="360">
        <el-table-column type="index" />
        <el-table-column prop="full_url" label="Invoke Uri" show-overflow-tooltip width="360"/>
        <el-table-column prop="namespace" label="命名空间" sortable show-overflow-tooltip  width="180"/>
        <el-table-column prop="refname" label="名称" sortable show-overflow-tooltip  width="160"/>
        <el-table-column prop="method" label="方法" sortable show-overflow-tooltip  width="120" />
        <el-table-column prop="success_count" sortable label="成功" />
        <el-table-column prop="failure_count" sortable label="失败" />
        <el-table-column prop="success_elapse" sortable label="总耗时(微秒)" width="160"/>
        <el-table-column prop="max_elapse" sortable label="最慢" />
        <el-table-column prop="min_elapse" sortable label="最快" />
        <el-table-column prop="avg_elapse" sortable label="平均" />
    </el-table>
  </template>
  
  <script>
  import resize from './mixins/resize'
  import * as datetime from '@/utils/datetime.js'
  
  export default {
    mixins: [resize],
    props: {
      chartData: {
        type: Array,
        required: true
      }
    },
    data() {
      return {
      }
    },
    mounted() {
    },
    beforeDestroy() {
    },
    methods: {
    }
  }
  </script>
  