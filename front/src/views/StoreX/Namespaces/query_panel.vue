<template>
    <div class="container">
        <add-hook :visible="showHookDialog" :hook="currentHook" @update:visible="handleHookDialogVisibleChange" @update:hook="handleUpdateHook" />
        <el-scrollbar>
            <el-form label-width="100px" :inline="false">
                <el-form-item label="引用名称">
                    <el-input v-model="data.name" />
                </el-form-item>
                <el-form-item label="支持分页查询">
                    <el-switch v-model="data.pagable" />
                </el-form-item>
                <el-form-item label="查询体">
                    <el-tabs v-model="activeName" type="border-card" style="width: 100%">
                        <el-tab-pane label="查询" name="query">
                            <el-input v-model="data.query_body" type="textarea" style="width: 100%" :rows="8" />
                        </el-tab-pane>
                        <el-tab-pane label="Count查询" name="count_query" :disabled="!data.pagable">
                            <el-input v-model="data.count_query" type="textarea" style="width: 100%" :rows="8" />
                        </el-tab-pane>
                    </el-tabs>
                </el-form-item>
                <el-form-item label="对象缓存">
                  <el-switch v-model="data.enable_cache" />
                  <el-form-item v-if="data.enable_cache" label="缓存时间">
                    <el-input v-model="data.cache_time" placeholder="缓存时间（单位：秒）"/>
                  </el-form-item>
                </el-form-item>
                <el-form-item label="功能权限">
                  <el-select v-model="data.perm_roles" multiple collapse-tags collapse-tags-tooltip placeholder="选择赋予该功能可访问的角色" style="width: 240px">
                    <el-option value="" label="全部角色">全部角色</el-option>
                    <el-option v-for="r in auth_roles" :value="r" :label="r">{{ r }}</el-option>
                  </el-select>
                </el-form-item>                
                <el-form-item label="数据权限">
                  <el-switch v-model="data.data_permission" />
                  <el-form-item label="授权字段">
                    <el-input :disabled="!data.data_permission" v-model="data.permission_field" placeholder="本对象中用于表示数据行级权限的字段"/>
                  </el-form-item>
                  <el-form-item label="关联字段">
                    <el-input :disabled="!data.data_permission" v-model="data.relative_field" placeholder="关联权限表的字段（可选），如果没有指定，则使用权限中的设定"/>
                  </el-form-item>
                </el-form-item>
                <el-form-item label="">
                  <div v-if="data.data_permission" class="remark">
                    启用数据权限后，需要在查询中合适的位置加入${DATA_PERMISSION_SQL}这个占位符。
                    ${DATA_PERMISSION_SQL}的值大概为：INNER JOIN tbl_data_permit __p ON a.user_id = __p.user_id and __p.user_id = ?
                    其中a.user_id为授权字段，是自定义查询中某个表的一个字段，关联字段为关联到数据权限表的某个字段。
                  </div>
                </el-form-item>
                <el-form-item label="Hook"  prop="hooks">
                    <el-table :data="data.hooks">
                        <el-table-column type="expand">
                            <template #default="h">
                                <code>
                                    {{ h.row.script }}
                                </code>
                            </template>
                        </el-table-column>
                        <el-table-column prop="lang" label="脚本语言" />
                        <el-table-column prop="event" label="事件" />
                        <el-table-column prop="before" label="前置处理" />
                        <el-table-column label="操作" width="100px">
                            <template #default="scoped">
                                <el-button type="primary" icon="Edit" circle @click="handleModifyHook(scoped.row)" />
                                <el-popconfirm title="确认要删除吗?" @confirm="handleRemoveHook(scoped.row)">
                                    <template #reference>
                                        <el-button type="danger" icon="Delete" circle />
                                    </template>
                                </el-popconfirm>
                            </template>
                        </el-table-column>
                    </el-table>
                    <el-button @click="onAddHook">添加</el-button>
                </el-form-item>
            </el-form>
            <el-tabs v-model="activeTable" class="demo-tabs">
              <el-tab-pane label="参数" name="params">
                <el-table :data="data.params" :border="true" :max-height="300">
                  <el-table-column fixed label="字段名" prop="field_name">
                      <template #header>
                        字段名
                        <el-tooltip class="box-item" effect="dark" placement="top-start" content="对应于查询体中的参数名称。在查询体中参数名称是唯一的。">
                          <el-icon><InfoFilled /></el-icon>
                        </el-tooltip>
                      </template>                    
                      <template #default="scoped">
                          <el-input v-model="scoped.row.field_name" />
                      </template>                
                  </el-table-column>
                  <el-table-column fixed label="属性名" prop="prop_name">
                      <template #header>
                        属性名
                        <el-tooltip class="box-item" effect="dark" placement="top-start" content="对应于如何访问参数体（JSON表示）的属性，多个不同的参数名可以对应一个相同的属性名，从而可以拿到相同的值。">
                          <el-icon><InfoFilled /></el-icon>
                        </el-tooltip>
                      </template>                    
                      <template #default="scoped">
                          <el-input v-model="scoped.row.prop_name" />
                      </template>
                  </el-table-column>
                  <el-table-column label="标题" prop="title">
                      <template #default="scoped">
                          <el-input v-model="scoped.row.title" />
                      </template>
                  </el-table-column>
                  <el-table-column label="固定参数" prop="pkey">
                      <template #header>
                        固定参数
                        <el-tooltip class="box-item" effect="dark" placement="top-start" content="该参数如果出现在查询体中，则必须声明为固定参数。">
                          <el-icon><InfoFilled /></el-icon>
                        </el-tooltip>
                      </template>
                      <template #default="scoped">
                          <el-checkbox v-model="scoped.row.pkey" />
                      </template>
                  </el-table-column>
                  <el-table-column label="类型" prop="col_type">
                      <template #default="scoped">
                          <el-input v-model="scoped.row.col_type" />
                      </template>
                  </el-table-column>
                  <el-table-column label="操作" width="60px">
                      <template #default="scoped">
                          <el-popconfirm title="确认要删除吗?" @confirm="onDelParam(scoped.row)">
                              <template #reference>
                                  <el-button type="danger" icon="Delete" circle />
                              </template>
                          </el-popconfirm>
                      </template>
                  </el-table-column>                
                </el-table>
                <el-button @click="onAddParam">添加参数</el-button>
              </el-tab-pane>
              <el-tab-pane label="数据定义" name="fields">
                <el-table :data="data.fields" :border="true" :max-height="300">
                  <el-table-column fixed label="字段名" prop="field_name">
                      <template #default="scoped">
                          <el-input v-model="scoped.row.field_name" />
                      </template>                
                  </el-table-column>
                  <el-table-column fixed label="属性名" prop="prop_name">
                      <template #default="scoped">
                          <el-input v-model="scoped.row.prop_name" />
                      </template>
                  </el-table-column>
                  <el-table-column label="标题" prop="title">
                      <template #default="scoped">
                          <el-input v-model="scoped.row.title" />
                      </template>
                  </el-table-column>
                  <el-table-column label="主键" prop="pkey">
                      <template #default="scoped">
                          <el-checkbox v-model="scoped.row.pkey" />
                      </template>
                  </el-table-column>
                  <el-table-column label="类型" prop="col_type">
                      <template #default="scoped">
                          <el-select v-model="scoped.row.col_type">
                            <el-option value="" label="原始类型">原始类型</el-option>
                            <el-option value="string"  label="String">String</el-option>
                            <el-option value="integer"  label="整型长整型">整型长整型</el-option>
                            <el-option value="double"  label="浮点型">浮点型</el-option>
                            <el-option value="bool"  label="布尔型">布尔型</el-option>
                            <el-option value="date"  label="日期">日期</el-option>
                            <el-option value="time"  label="时间">时间</el-option>
                            <el-option value="datetime"  label="日期时间">日期时间</el-option>
                            <el-option value="binnary"  label="二进制">二进制</el-option>
                            <el-option value="json"  label="JSON">JSON</el-option>
                        </el-select>
                      </template>
                  </el-table-column>
                  <el-table-column label="字段类型" prop="field_type" width="140px">
                    <template #header>
                      字段类型
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="字段类型为数据库表示的原始类型，该类型可能用于与所在数据库相关，用于辅助类型转换。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-input v-model="scoped.row.field_type" />
                    </template>
                  </el-table-column>
                  <el-table-column label="Base64" prop="base64">
                    <template #default="scoped">
                        <el-checkbox v-model="scoped.row.base64" :disabled="scoped.row.col_type !== 'binnary'"/>
                    </template>
                </el-table-column>
                <el-table-column label="脱敏" prop="desensitize">
                    <template #default="scoped">
                        <el-select v-model="scoped.row.desensitize" :disabled="!(scoped.row.col_type === 'string' || scoped.row.col_type === 'String' || scoped.row.col_type === 'varchar' || scoped.row.col_type === 'str' || scoped.row.col_type === 'text')">
                            <el-option value="none" label="不作处理">不作处理</el-option>
                            <el-option value="aes"  label="AES加密">AES加密</el-option>
                            <el-option value="rsa"  label="RSA加密">RSA加密</el-option>
                            <el-option value="base64"  label="Base64">Base64</el-option>
                            <el-option value="replace"  label="替换部分值">替换部分值</el-option>
                            <el-option value="null"  label="返回空">返回空</el-option>
                        </el-select>
                    </template>
                </el-table-column>
                  <el-table-column label="操作" width="60px">
                      <template #default="scoped">
                          <el-popconfirm title="确认要删除吗?" @confirm="onDelField(scoped.row)">
                              <template #reference>
                                  <el-button type="danger" icon="Delete" circle />
                              </template>
                          </el-popconfirm>
                      </template>
                  </el-table-column>                
                </el-table>
                <el-button @click="onAddField">添加字段</el-button>
              </el-tab-pane>
            </el-tabs>
        </el-scrollbar>
      <div class="drawer-footer">
        <el-button @click="$emit('update:visible', false)">关闭</el-button>
        <el-button type="primary" @click="onConfirm">
          保存
        </el-button>
        <el-popconfirm
            confirmButtonText="确定"
            cancelButtonText="取消"
            icon="el-icon-info"
            iconColor="red"
            width="280px"
            title="你确认要删除该查询服务的定义吗？"
            @confirm="handleRemove"
        >
            <template #reference>
            <el-button type="danger">删除</el-button>
            </template>
        </el-popconfirm>
      </div>
    </div>
</template>
  
  <script lang="ts" setup name="config">
  import { update, remove, lang_list, authorize_roles_get } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import AddHook from "./add_hook.vue"
  import { mergeProps, onMounted, ref, watch } from "vue";
  const props = defineProps<{ data: any }>();
  const emit = defineEmits(['update:data', 'update:visible'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const ScriptLangs = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const activeName = ref<any>("query")
  const activeTable = ref<any>("params")
  const activeHookName = ref<any>("uri")
  const showHookDialog = ref<boolean>(false)
  const currentHook = ref<any>()
  const auth_roles = ref<Array<any>>([])

  function handleUpdate() {
    var ns = route.query.ns as string  
    update(ns, 'query', [props.data]).then(res => {
      emit("update:visible", false)
      emit("update:data", true)
    }).catch(ex => {
      console.log(ex)
    })
  }

  function handleRemove() {
    var ns = route.query.ns as string  
    remove(ns, 'query', [props.data.name]).then(res => {
      emit("update:visible", false)
      emit("update:data", true)
    }).catch(ex => {
      console.log(ex)
    })
  }
  
  function fetchAuthRoles() {
    var ns = route.query.ns as string  
    authorize_roles_get().then(res => {
      auth_roles.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  }  
  
  function handleHookDialogVisibleChange(e: any) {
    showHookDialog.value = e
  }  

  function onAddHook() {
    showHookDialog.value = true
    currentHook.value = {}
  }

  function handleUpdateHook(hk) {
    let hooks = props.data.hooks
    if (!hooks) {
        hooks = []
    }
    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    if (index >= 0) {
      hooks.splice(index, 1)
    }
    hooks.push(hk)
    props.data.hooks = hooks
  }

  function handleRemoveHook(hk) {
    let hooks = props.data.hooks
    if (!hooks) {
        hooks = []
    }
    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    hooks.splice(index, 1)
    props.data.hooks = hooks
  }

  function handleModifyHook(hk) {
    currentHook.value = hk
    showHookDialog.value = true
  }

  
  function handleSelectionChange(e: any) {
    console.log(e)
    selections.value = e
  }
  
  function onConfirm() {
    handleUpdate()
  }

  function fetchLang(){
    lang_list().then(res => {
      if (res.status === 0 || res.status === 200) {
        ScriptLangs.value = res.data
      }
    })
  }

  function onAddParam(){
    if (props.data) {
      if (props.data.params) {
        if (props.data.params.length >= 0) {
          var params = props.data.params
          params.push({})
          props.data.params = params
        } else {
          props.data.params = [{}]
        }
      } else {
        props.data.params = [{}]
      }
    } 
  }

  function onDelParam(row: any) {
    if (props.data) {
      if (props.data.params) {
        var params = props.data.params
        let index = params.indexOf(row) // 找到要删除的元素的索引，此处为 2
        params.splice(index, 1)
        props.data.params = params
      }
    }
  }
  
  function onAddField(){
    if (props.data) {
      if (props.data.fields) {
        if (props.data.fields.length >= 0) {
          var fields = props.data.fields
          fields.push({})
          props.data.fields = fields
        } else {
          props.data.fields = [{}]
        }
      } else {
        props.data.fields = [{}]
      }
    } 
  }

  function onDelField(row: any) {
    if (props.data) {
      if (props.data.fields) {
        var fields = props.data.fields
        let index = fields.indexOf(row) // 找到要删除的元素的索引，此处为 2
        fields.splice(index, 1)
        props.data.fields = fields
      }
    }
  }

  onMounted(() => {
      fetchAuthRoles()
      console.log("config");
      fetchLang()
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  