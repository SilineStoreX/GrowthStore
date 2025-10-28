<template>
    <el-table :data="chartData" ref="connections" row-key="db_url" :border="true" max-height="240">
        <el-table-column type="index" />
        <el-table-column prop="db_url" label="数据库连接池" show-overflow-tooltip width="360"/>
        <el-table-column prop="namespace" label="命名空间" sortable show-overflow-tooltip  width="180"/>
        <el-table-column prop="max_open" label="最大连接数" sortable show-overflow-tooltip  width="160"/>
        <el-table-column prop="connections" label="连接数" sortable show-overflow-tooltip  width="120" />
        <el-table-column prop="idle" sortable label="空闲" />
        <el-table-column prop="in_use" sortable label="正在使用" />
        <el-table-column prop="waits" sortable label="等待" width="160"/>
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
  