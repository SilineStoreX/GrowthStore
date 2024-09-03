<template>
    <div class="home">
      <div>
        <el-form label-width="100px" :inline="true">
          <el-form-item label="Redis Key">
              <el-input v-model="cmdfrm.key_prefix"></el-input>
          </el-form-item>
          <el-form-item>
              <el-button @click="onQueryKeys">查询</el-button>
              <el-button @click="onDelKeys">删除</el-button>              
              <el-button @click="onFlushAll">清空</el-button>
          </el-form-item>
        </el-form>
      </div>
      <el-table :data="rediskeys" ref="redistable" :expand-row-keys="expandedkeys" row-key="key" @expand-change="onTableRowExpanded">
          <el-table-column type="expand">
              <template #default="scoped">
                  <JsonViewer :value="scoped.row.body" :expand-depth="5" copyable boxed sort></JsonViewer>
              </template>
          </el-table-column>
          <el-table-column prop="key" label="Redis Key" />
          <el-table-column label="操作" width="60px">
              <template #default="scoped">
                  <el-popconfirm title="确认要删除吗?" @confirm="onDelKey(scoped.row.key)">
                      <template #reference>
                          <el-button type="danger" icon="Delete" circle />
                      </template>
                  </el-popconfirm>
              </template>
          </el-table-column>
        </el-table>
    </div>
  </template>
  
  <script lang="ts" setup name="redis">
  import { onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import JsonViewer from 'vue-json-viewer'
  import { redis_del, redis_get, redis_keys, redis_flushall } from "@/http/modules/management";

  const route = useRoute()
  const rediskeys = ref<Array<any>>([])
  const cmdfrm = ref<any>({})
  const expandedkeys = ref<Array<any>>([])

  function onDelKeys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    onDelKey(key)
  }

  function onDelKey(key) {
    var ns = route.query.ns as string
    redis_del(ns, key).then(res => {
      fetch_redis_keys()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetch_redis_keys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    redis_keys(ns, key).then(res => {
      rediskeys.value = res.data.map((d) => {
        return {
          key: d,
          body: ''
        }
      })
      expandedkeys.value = []
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onQueryKeys() {
    fetch_redis_keys()
  }

  function onFlushAll() {
    var ns = route.query.ns as string
    redis_flushall(ns).then(res => {
      fetch_redis_keys()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onTableRowExpanded(row: any, expanded: any[]) {
    console.log('ex', expanded)
    if (expanded.length > 0) {
      var ns = route.query.ns as string
      redis_get(ns, row.key).then(res => {
          row.body = res.data
          expandedkeys.value = [row.key]
      });
    }
  }

  onMounted(() => {
    console.log("config");
    fetch_redis_keys()
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  