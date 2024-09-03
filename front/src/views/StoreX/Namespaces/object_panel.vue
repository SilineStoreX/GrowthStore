<template>
    <div class="container">
      <add-hook :visible="showHookDialog" :hook="currentHook" @update:visible="handleHookDialogVisibleChange" @update:hook="handleUpdateHook" />
      <el-scrollbar>
        <el-form label-width="100px" :inline="false">
            <el-form-item label="引用名称">
                <el-input v-model="data.name" />
            </el-form-item>
            <el-form-item label="对象名称">
                <el-input disabled v-model="data.object_name" />
            </el-form-item>
            <el-form-item label="对象类型">
                <el-input disabled v-model="data.object_type" />
            </el-form-item>
            <el-form-item label="数据校验">
                <el-switch v-model="data.validation" />
                <el-form-item label="不完全验证"><el-switch v-model="data.parti_valid" /></el-form-item>
            </el-form-item>
            <el-form-item label="对象缓存">
                <el-switch v-model="data.enable_cache" />
                <el-form-item v-if="data.enable_cache" label="缓存时间">
                  <el-input v-model="data.cache_time" placeholder="缓存时间（单位：秒）"/>
                </el-form-item>
            </el-form-item>
            <el-form-item label="功能权限">
                <el-form-item label="读授权">
                  <el-select v-model="data.read_perm_roles" multiple collapse-tags collapse-tags-tooltip placeholder="选择赋予该功能可读的角色" style="width: 240px">
                    <el-option value="" label="全部角色可读">全部角色可读</el-option>
                    <el-option v-for="r in auth_roles" :key="r" :value="r" :label="r">{{ r }}</el-option>
                  </el-select>
                </el-form-item>
                <el-form-item label="写授权">
                  <el-select v-model="data.write_perm_roles" multiple collapse-tags collapse-tags-tooltip placeholder="选择赋予该功能可写的角色" style="width: 240px">
                    <el-option value="" label="全部角色可写">全部角色可写</el-option>
                    <el-option v-for="r in auth_roles" :key="r" :value="r" :label="r">{{ r }}</el-option>
                  </el-select>
                </el-form-item>
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
        </el-form>
        <el-tabs v-model="activeName" class="demo-tabs">
          <el-tab-pane label="数据结构" name="data">
            <el-table :data="data.fields" :border="true">
                <el-table-column fixed label="字段名" prop="field_name"  width="160px">
                    <template #header>
                      字段名
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="对应于该表的字段名称">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-input v-model="scoped.row.field_name" />
                    </template>
                </el-table-column>
                <el-table-column fixed label="属性名" prop="prop_name"  width="160px">
                    <template #header>
                      属性名
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="在生成JSON时，产生JSON的属性名称，从而替代原始字段名。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-input v-model="scoped.row.prop_name" />
                    </template>
                </el-table-column>
                <el-table-column label="标题" prop="title" width="160px">
                    <template #header>
                      标题
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="用于在界面（UI）上的对应显示。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-input v-model="scoped.row.title" />
                    </template>
                </el-table-column>
                <el-table-column label="主键" prop="pkey"  width="60px">
                    <template #default="scoped">
                        <el-checkbox v-model="scoped.row.pkey" />
                    </template>
                </el-table-column>
                <el-table-column label="类型" prop="col_type"  width="140px">
                    <template #default="scoped">
                        <el-select v-model="scoped.row.col_type">
                            <el-option value="" label="原始类型">原始类型</el-option>
                            <el-option value="string"  label="String">String</el-option>
                            <el-option value="integer"  label="整型">整型</el-option>
                            <el-option value="double"  label="浮点型">浮点型</el-option>
                            <el-option value="bool"  label="布尔型">布尔型</el-option>
                            <el-option value="date"  label="日期">日期</el-option>
                            <el-option value="time"  label="时间">时间</el-option>
                            <el-option value="datetime"  label="日期时间">日期时间</el-option>
                            <el-option value="binnary"  label="二进制">二进制</el-option>
                            <el-option value="relation"  label="关联表">关联表</el-option>
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
                <el-table-column label="生成器" prop="generator"  width="140px">
                    <template #header>
                      生成器
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="用于在新增或修改时自动对该字段进行值生成的配置。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-select v-model="scoped.row.generator">
                            <el-option value="" label="无">无</el-option>
                            <el-option value="autoincrement"  label="自增">自增</el-option>
                            <el-option value="snowflake"  label="雪花ID">雪花ID</el-option>
                            <el-option value="uuid"  label="UUID">UUID</el-option>
                            <el-option value="cur_user_id"  label="当前用户ID（新增）">当前用户ID（新增）</el-option>
                            <el-option value="cur_user_name"  label="当前用户名（新增）">当前用户名（新增）</el-option>
                            <el-option value="mod_user_id"  label="当前用户ID（新增/修改）">当前用户ID（新增/修改）</el-option>
                            <el-option value="mod_user_name"  label="当前用户名（新增/修改）">当前用户名（新增/修改）</el-option>                           
                            <el-option value="cur_datetime"  label="当前日期时间（新增）">当前日期时间（新增）</el-option>
                            <el-option value="cur_date"  label="当前日期（新增）">当前日期（新增）</el-option>
                            <el-option value="cur_time"  label="当前时间（新增）">当前时间（新增）</el-option>
                            <el-option value="mod_datetime"  label="当前日期时间（新增/修改）">当前日期时间（新增/修改）</el-option>
                            <el-option value="mod_date"  label="当前日期（新增/修改）">当前日期（新增/修改）</el-option>
                            <el-option value="mod_time"  label="当前时间（新增/修改）">当前时间（新增/修改）</el-option>                            
                        </el-select>
                    </template>
                </el-table-column>
                <el-table-column label="关联对象" prop="relation_object"  width="100px">
                  <template #header>
                    关联对象
                    <el-tooltip class="box-item" effect="dark" placement="top-start" content="关联到目标对象，可以表示为1..1关系，1..N关系，以及N..N关系的目标对象。">
                      <el-icon><InfoFilled /></el-icon>
                    </el-tooltip>
                  </template>
                  <template #default="scoped">
                        <el-input :disabled="scoped.row.col_type !== 'relation'" v-model="scoped.row.relation_object" />
                    </template>
                </el-table-column>
                <el-table-column label="关联字段" prop="relation_field"  width="100px">
                  <template #header>
                    关联字段
                    <el-tooltip class="box-item" effect="dark" placement="top-start" content="关联到目标对象的关联字段">
                      <el-icon><InfoFilled /></el-icon>
                    </el-tooltip>
                  </template>
                  <template #default="scoped">
                        <el-input :disabled="scoped.row.col_type !== 'relation'" v-model="scoped.row.relation_field" />
                    </template>
                </el-table-column>
                <el-table-column label="数组" prop="relation_array"  width="90px">
                  <template #header>
                    数组 
                    <el-tooltip class="box-item" effect="dark" placement="top-start" content="表示相对于目标对象的多(N)关系，如1..N，N..N关系。">
                      <el-icon><InfoFilled /></el-icon>
                    </el-tooltip>
                  </template>
                  <template #default="scoped">
                        <el-switch :disabled="scoped.row.col_type !== 'relation'" v-model="scoped.row.relation_array" />
                    </template>
                </el-table-column>
                <el-table-column label="N..N关系" prop="relation_middle"  width="160px">
                  <template #header>
                    N..N关系 
                    <el-tooltip class="box-item" effect="dark" placement="top-start" content="N..N关系时，需要中间表辅助，此处填写中间表表达式，即，<中间表>.<中间表到目标表的关联字段>">
                      <el-icon><InfoFilled /></el-icon>
                    </el-tooltip>
                  </template>
                  <template #default="scoped">
                        <el-input :disabled="!(scoped.row.col_type === 'relation' && scoped.row.relation_array)" v-model="scoped.row.relation_middle" />
                    </template>
                </el-table-column>
                <el-table-column label="长度" prop="col_length">
                    <template #default="scoped">
                        <el-input v-model="scoped.row.col_length" />
                    </template>
                </el-table-column>
                <el-table-column label="验证" prop="validation"  width="160px">
                    <template #header>
                      验证
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="用于在新增或修改时进行对值进行验证，通常以正则表达式来表示。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-input v-model="scoped.row.validation" />
                    </template>
                </el-table-column>
                <el-table-column label="Base64" prop="base64" width="100px">
                    <template #header>
                      Base64
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="当列类型为Binnary时，是否以Base64字符串来表示">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>
                    <template #default="scoped">
                        <el-checkbox v-model="scoped.row.base64" :disabled="scoped.row.col_type !== 'binnary'"/>
                    </template>
                </el-table-column>
                <el-table-column label="脱敏" prop="desensitize"  width="150px">
                    <template #header>
                      脱敏
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="对于字符串信息，是否通过脱敏算法来将这些内容进行改变。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>                  
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
                <el-table-column label="加密存储" prop="crypto_store" width="100px">
                    <template #header>
                      加密存储
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="该值为True时，该字段的内容先加密再存储。且，只有在脱敏算法为AES、RSA、Base64时生效。启用加密存储后，需要确保该字段的长度足够长，能够保存加密后的字符串。同时，由于RSA加密对消息长度有限制（大约为53个字符），如需要对超长的内容进行加密时，请选择AES算法。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>                  
                    <template #default="scoped">
                        <el-checkbox v-model="scoped.row.crypto_store" />
                    </template>
                </el-table-column>
                <el-table-column label="详情字段" prop="detail_only" width="100px">
                    <template #header>
                      详情字段
                      <el-tooltip class="box-item" effect="dark" placement="top-start" content="该值为True时，此字段不在Query/Paged_Query的结果中显示。">
                        <el-icon><InfoFilled /></el-icon>
                      </el-tooltip>
                    </template>                  
                    <template #default="scoped">
                        <el-checkbox v-model="scoped.row.detail_only" />
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
            <el-button @click="onAddField" style="margin-top: 10px;">添加字段</el-button>
          </el-tab-pane>
          <el-tab-pane label="新增时Hook" name="insert">
            <span>在新增记录时执行该Hook，如insert和upsert</span>
            <el-table :data="data.insert_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="修改时Hook" name="update">
            <span>在修改记录时执行该Hook，如update/upsert/save_batch/update_by</span>
            <el-table :data="data.update_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="批量保存Hook" name="save_batch">
            <span>在执行批量保存时执行该Hook，如save_batch</span>
            <el-table :data="data.savebatch_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="删除时Hook" name="delete">
            <span>在删除记录时执行该Hook，如delete和delete_by</span>
            <el-table :data="data.delete_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="UPSERT时Hook" name="upsert">
            <span>在执行UPSERT操作时执行该Hook，UPSERT是INSERT和UPDATE操作的复合体，根据被处理对象的指定条件来判定记录是否存在，从而执行INSERT或UPDATE操作</span>
            <el-table :data="data.upsert_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="唯一查找的Hook" name="select">
            <span>在进行select/find_one操作时执行该Hook</span>
            <el-table :data="data.select_hooks">
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
          </el-tab-pane>
          <el-tab-pane label="查询时Hook" name="query">
            <span>在查询时执行该Hook，需要注意是单记录查询，还是多记录查询，以及分页查询</span>
            <el-table :data="data.query_hooks">
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
  import { update, remove, authorize_roles_get } from "@/http/modules/management";
  import { useRoute } from "vue-router";
  import AddHook from "./add_hook.vue"
  import { mergeProps, onMounted, ref, watch } from "vue";
  const props = defineProps<{ data: any }>();
  const emit = defineEmits(['update:data', 'update:visible'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const activeName = ref<string>("data")
  const showHookDialog = ref<boolean>(false)
  const currentHook = ref<any>()
  const auth_roles = ref<Array<any>>([])

  function handleUpdate() {
    var ns = route.query.ns as string  
    update(ns, 'object', [props.data]).then(res => {
      emit("update:visible", false)
      emit("update:data", true)
    }).catch(ex => {
      console.log(ex)
    })
  }

  function handleRemove() {
    var ns = route.query.ns as string  
    remove(ns, 'object', [props.data.name]).then(res => {
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

  function handleSelectionChange(e: any) {
    console.log(e)
    selections.value = e
  }
  
  function onConfirm() {
    handleUpdate()
  }

  function onAddHook() {
    showHookDialog.value = true
    currentHook.value = {}
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

  function handleUpdateHook(hk: any) {
    let current = activeName.value
    if (current !== 'insert' && current !== 'update' && current !== 'delete' && current !== 'query' && current !== 'upsert' && current !== 'select' && current !== 'save_batch') {
      return
    }

    let hooks = current === 'insert' ? props.data.insert_hooks : (current === 'update' ? props.data.update_hooks : (current === 'delete' ? props.data.delete_hooks : (current === 'query' ? props.data.query_hooks : (current === 'select' ? props.data.select_hooks : (current === 'upsert' ? props.data.upsert_hooks : (current === 'save_batch' ? props.data.savebatch_hooks : undefined))))))
    if (!hooks) {
        hooks = []
    }
    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    if (index >= 0) {
      hooks.splice(index, 1)
    }
    hooks.push(hk)
    if (current === 'insert') {
      props.data.insert_hooks = hooks
    } else if (current === 'update') {
      props.data.update_hooks = hooks
    } else if (current === 'delete') {
      props.data.delete_hooks = hooks
    } else if (current === 'query') {
      props.data.query_hooks = hooks
    } else if (current === 'upsert') {
      props.data.upsert_hooks = hooks
    } else if (current === 'select') {
      props.data.select_hooks = hooks
    } else if (current === 'save_batch') {
      props.data.savebatch_hooks = hooks
    }
  }

  function handleRemoveHook(hk: any) {
    let current = activeName.value
    if (current !== 'insert' && current !== 'update' && current !== 'delete' && current !== 'query' && current !== 'upsert' && current !== 'select' && current !== 'save_batch') {
      return
    }

    let hooks = current === 'insert' ? props.data.insert_hooks : (current === 'update' ? props.data.update_hooks : (current === 'delete' ? props.data.delete_hooks : (current === 'query' ? props.data.query_hooks : (current === 'select' ? props.data.select_hooks : (current === 'upsert' ? props.data.upsert_hooks : (current === 'save_batch' ? props.data.savebatch_hooks : undefined))))))    
    if (!hooks) {
        hooks = []
    }
    let index = hooks.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    hooks.splice(index, 1)
    if (current === 'insert') {
      props.data.insert_hooks = hooks
    } else if (current === 'update') {
      props.data.update_hooks = hooks
    } else if (current === 'delete') {
      props.data.delete_hooks = hooks
    } else if (current === 'query') {
      props.data.query_hooks = hooks
    } else if (current === 'upsert') {
      props.data.upsert_hooks = hooks
    } else if (current === 'select') {
      props.data.select_hooks = hooks
    } else if (current === 'save_batch') {
      props.data.savebatch_hooks = hooks
    }
  }

  function handleModifyHook(hk: any) {
    currentHook.value = hk
    showHookDialog.value = true
  }  
  
  onMounted(() => {
      fetchAuthRoles()
      console.log("config");
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  