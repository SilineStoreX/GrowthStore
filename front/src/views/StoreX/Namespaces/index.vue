<template>
    <div class="home">
        <AddSchema :visible="showAddSchemaDialog" @update:visible="handleVisibleChange" />
        <AddPlugin :visible="showAddPlugin" @update:visible="handlePluginVisibleChange" @datasync="onReloadService"/>
        <el-collapse v-model="activeNames" @change="handleChange" class="opblock-post">
            <el-collapse-item title="基础信息" name="info">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Avatar /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>基础信息</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>              
                <el-form label-width="180px">
                    <el-form-item label="文件名">
                        <el-input disabled v-model="conf.filename" />
                    </el-form-item>
                    <el-form-item label="Namespace">
                        <el-input disabled v-model="conf.namespace" />
                    </el-form-item>
                </el-form>
            </el-collapse-item>
            <el-collapse-item title="配置" name="config" class="opblock-post">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Operation /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>配置</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>                
                <el-form label-width="180px">
                    <el-form-item label="数据库连接">
                        <el-input v-model="conf.db_url" />
                    </el-form-item>
                    <el-form-item label="遗忘Timezone">
                        <el-switch v-model="conf.relaxy_timezone" />
                        <span style="padding-left: 20px;">在处理Date/Time相关类型时，不将Timezone标识返回</span>
                    </el-form-item>
                    <el-form-item label="Redis连接">
                        <el-input v-model="conf.redis_url" style="width: calc(50%)" />
                        <el-form-item label="最大连接数">
                            <el-input v-model="conf.max_redis_pool" />
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="AES加密密钥">
                        <el-input v-model="conf.aes_key" />
                    </el-form-item>
                    <el-form-item label="AES盐">
                        <el-input v-model="conf.aes_solt" />
                    </el-form-item>
                    <el-form-item label="RSA Public Key">
                        <el-input v-model="conf.rsa_public_key" type="textarea" :rows="5" />
                    </el-form-item>
                    <el-form-item label="RSA Private Key">
                        <el-input v-model="conf.rsa_private_key" type="textarea" :rows="5" />
                    </el-form-item> 
                    <el-form-item label="上传文件存放路径">
                        <el-input v-model="conf.upload_filepath" />
                    </el-form-item>
                    <el-form-item label="采用日期方式建立子文件夹">
                        <el-switch v-model="conf.subfolder_bydate" />
                        <el-form-item label="采用类型建立子文件夹">
                            <el-switch v-model="conf.subfolder_bytype" />
                        </el-form-item>
                        <el-form-item label="上传文件最大尺寸(MB)">
                            <el-input v-model="conf.max_filesize" type="number" />
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="直接下载">
                        <el-switch v-model="conf.download_direct" />
                        <el-form-item label="文件下载前缀">
                        <el-input v-model="conf.download_prefix" placeholder="下载文件是添加的前缀，有些使用nginx来代理下载的，则应该为nginx公布出来的URL" style="width: 400px"/>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="文件查询URI">
                        <el-input v-model="conf.download_query" placeholder="提供用于按file_id进行查询的InvokeURI" style="width: 300px"/>
                        <el-form-item label="文件名字段">
                            <el-input v-model="conf.download_file_name" placeholder="提供返回结果中用于表示文件名称的字段" style="width: 300px"/>
                        </el-form-item>
                        <el-form-item label="文件存放路径字段">
                            <el-input v-model="conf.download_file_path" placeholder="提供返回结果中用于表示文件存放路径的字段" style="width: 300px"/>
                        </el-form-item>
                    </el-form-item>
                    <el-form-item label="同时上传到OSS">
                        <el-switch v-model="conf.upload_to_oss" />
                    </el-form-item>
                    <el-form-item v-if="conf.upload_to_oss" label="OSS Endpoint">
                        <el-input v-model="conf.oss_endpoint" />
                    </el-form-item>
                    <el-form-item v-if="conf.upload_to_oss" label="OSS BulkId">
                        <el-input v-model="conf.oss_bulk_id" />
                    </el-form-item>
                    <el-form-item v-if="conf.upload_to_oss" label="OSS RAM">
                        <el-input v-model="conf.oss_auth" />
                    </el-form-item>
                </el-form>
            </el-collapse-item>
            <el-collapse-item title="存储服务" name="objects" class="opblock-post">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Basketball /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>存储服务</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>                
                <el-table :data="conf.objects" @row-click="openTypicalDraw('object', $event)">
                    <el-table-column type="expand">
                        <template #default="scoped">
                            <el-table :data="scoped.row.fields" :border="true">
                                <el-table-column label="字段名" prop="field_name" />
                                <el-table-column label="属性名" prop="prop_name" />
                                <el-table-column label="标题" prop="title" />
                                <el-table-column label="主键" prop="pkey" />
                                <el-table-column label="二进制类型" prop="binary" />
                                <el-table-column label="Base64编码" prop="base64" />
                                <el-table-column label="关联对象" prop="relation_object"/>
                                <el-table-column label="关联字段" prop="relation_field"/>
                                <el-table-column label="数组" prop="relation_array"/>
                            </el-table>
                        </template>
                    </el-table-column>
                    <el-table-column prop="object_name" label="对象名称" width="180" />
                    <el-table-column prop="name" label="引用名称" width="180" />
                    <el-table-column prop="insert" label="新增" width="180">
                        <template #default="scoped">
                            {{ scoped.row.object_type === 'VIEW' ? 'No': 'Yes' }}
                        </template>
                    </el-table-column>
                    <el-table-column prop="update" label="修改" width="180">
                        <template #default="scoped">
                            {{ scoped.row.object_type === 'VIEW' ? 'No': 'Yes' }}
                        </template>
                    </el-table-column>                        
                    <el-table-column prop="delete" label="删除" width="180">
                        <template #default="scoped">
                            {{ scoped.row.object_type === 'VIEW' ? 'No': 'Yes' }}
                        </template>
                    </el-table-column>                        
                    <el-table-column prop="get_one" label="获取" width="180">
                        <template #default="scoped">
                            {{ scoped.row.fields && scoped.row.fields.length > 0 ? 'Yes': 'No' }}
                        </template>
                    </el-table-column>                        
                    <el-table-column prop="find_one" label="查找" width="180">
                        <template #default="scoped">
                            {{ scoped.row.fields && scoped.row.fields.length > 0 ? 'Yes': 'No' }}
                        </template>
                    </el-table-column>                        
                    <el-table-column prop="find" label="查询" width="180">
                        <template #default="scoped">
                            {{ scoped.row.fields && scoped.row.fields.length > 0 ? 'Yes': 'No' }}
                        </template>
                    </el-table-column>                        
                    <el-table-column prop="paged" label="分页">
                        <template #default="scoped">
                            {{ scoped.row.fields && scoped.row.fields.length > 0 ? 'Yes': 'No' }}
                        </template>
                    </el-table-column>
                </el-table>
                <div style="margin-top: 20px">
                    <el-button @click="onAddStoreObject">添加存储对象</el-button>
                    <el-button @click="onReloadService">刷新</el-button>
                </div>
            </el-collapse-item>
            <el-collapse-item title="查询服务" name="querys" class="opblock-post">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Football /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>查询服务</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>                
                <el-table ref="table_query" :data="conf.querys" @row-click="openTypicalDraw('query', $event)">
                    <el-table-column type="expand">
                        <template #default="scoped">
                            <el-table ref="table_query_field" :data="scoped.row.params" :border="true">
                                <el-table-column label="字段名" prop="field_name" />
                                <el-table-column label="属性名" prop="prop_name" />
                                <el-table-column label="标题" prop="title" />
                                <el-table-column label="必填参数" prop="pkey" />
                                <el-table-column label="二进制类型" prop="binary" />
                                <el-table-column label="Base64编码" prop="base64" />
                            </el-table>
                        </template>
                    </el-table-column>
                    <el-table-column prop="name" label="引用名称" width="180" />
                    <el-table-column prop="pagable" label="支持分页" width="80" />
                    <el-table-column prop="query_body" label="查询体" />
                </el-table>
                <div style="margin-top: 20px">
                    <el-button @click="onAddQueryService">添加查询服务</el-button>
                    <el-button @click="onReloadService">刷新</el-button>
                </div>
            </el-collapse-item>
            <el-collapse-item title="扩展功能" name="plugins">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Coin /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>扩展功能</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>                
                <el-table ref="plugin_list" :data="conf.plugins" @row-click="openTypicalDraw('plugin', $event)">
                    <el-table-column prop="name" label="引用名称" width="180" />
                    <el-table-column prop="protocol" label="插件协议" width="220" />
                    <el-table-column prop="config" label="配置文件" width="280" />
                    <el-table-column prop="enable" label="启用">
                        <template #default="scoped">
                            <el-switch v-model="scoped.row.enable" />
                        </template>
                    </el-table-column>
                </el-table>
                <div style="margin-top: 20px">
                    <el-button @click="onAddPlugin">添加扩展功能</el-button>
                    <el-button @click="onReloadService">刷新</el-button>
                </div>
            </el-collapse-item>
            <el-collapse-item title="归档与恢复" name="archive" class="opblock-put">
                <template #title>
                    <div class="opblock-summary-block">
                    <span class="opblock-summary-method"><el-icon><Suitcase /></el-icon></span>
                    </div>
                    <div class="opblock-summary-path-description-wrapper">
                    <span class="opblock-summary-path">
                        <a class="nostyle">
                        <span>归档与恢复</span>
                        </a>
                    </span>
                    <div class="opblock-summary-description"> </div>
                    </div>
                </template>
                <el-form label-position="top" label-width="auto" :inline="false">
                    <el-form-item label="配置与脚本归档">
                        <el-button type="primary" @click="onArchiveConfig" size="small">执行归档</el-button>
                        <span style="padding-left: 20px;">将所有的的配置文件与脚本文件进行归档。归档成功后，自动下载该归档文件。</span>
                    </el-form-item>
                    <el-form-item label="配置与脚本恢复">
                        <el-upload
                            class="upload-demo"
                            :headers="get_auth_header()"
                            :action="'/management/config/restore?force=' + forceReplace"
                            :show-file-list="false"
                            method="post"
                            accept=".zip, application/zip"
                            :on-success="handleOnUpload"
                        >
                            <el-button size="small" type="primary">点击上传</el-button>
                            <template #tip>
                            <div class="el-upload__tip">只能上传 zip 文件，且不超过 50MB</div>
                            </template>
                        </el-upload>
                        <span style="padding-left: 20px;">将归档的文件上传并进行恢复。</span>
                        <el-form-item style="width: 300px">
                            <span style="padding-left: 10px;">强制替换已存在的配置</span>
                            <el-switch v-model="forceReplace" />
                        </el-form-item>
                    </el-form-item>
                </el-form>
            </el-collapse-item>
        </el-collapse>
        <div class="button-container">
            <el-button type="primary" icon="Files" @click="onSaveConfig">保存</el-button>
            <el-popconfirm title="配置被删除后，相关的配置文件会被物理删除，你将无法恢复，建议你先使用“归档与恢复”功能将相关的配置打包备份。你还确认要删除吗?" :width="380" @confirm="onDeleteConfig">
                <template #reference>
                    <el-button type="danger" icon="Delete">删除</el-button>
                </template>
            </el-popconfirm>
        </div>
        <el-drawer title="详细配置" v-model="showDrawer" :with-header="false" :size="900">
            <ObjectPanel v-if="show_panel === 'object'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <QueryPanel v-if="show_panel === 'query'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <ComposePanel v-if="show_panel === 'compose'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <RestapiPanel v-if="show_panel === 'restapi'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <KafkaPanel v-if="show_panel === 'kafka'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <MqttPanel v-if="show_panel === 'mqtt'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <ElasticsearchPanel v-if="show_panel === 'elasticsearch'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
            <SyncTaskPanel v-if="show_panel === 'synctask'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />            
            <PluginPanel v-if="show_panel === 'plugin'" :data="select_object" @update:visible="handleDrawerClosed" @update:data="onReloadService" />
        </el-drawer>
    </div>
</template>
  
<script lang="ts" setup name="namespace">
import { onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import { fetchConfig, update, archive, configDelete } from "@/http/modules/management";
import AddSchema from "./add_schema.vue"
import AddPlugin from "./add_plugin.vue"
import ObjectPanel from "./object_panel.vue"
import QueryPanel from "./query_panel.vue"
import PluginPanel from "./plugin_panel.vue"
import ComposePanel from "./compose_panel.vue"
import RestapiPanel from "./restapi_panel.vue"
import KafkaPanel from "./kafka_panel.vue"
import MqttPanel from "./mqtt_panel.vue"
import SyncTaskPanel from "./sync_panel.vue"
import ElasticsearchPanel from  "./es_panel.vue"
import { onActivated, onUpdated } from "vue";
import { ElMessage } from "element-plus";
import { GlobalStore } from "@/stores";

const activeNames = ref<any>([]);
const conf = ref<any>({});
const route = useRoute();
const showAddSchemaDialog = ref<boolean>(false)
const showDrawer = ref<boolean>(false)
const showAddPlugin = ref<boolean>(false)
const select_object = ref<any>({})
const show_panel = ref<string>('')
const globalStore = GlobalStore();
const forceReplace = ref<boolean>(false);

function handleChange(e: any) {
    console.log(e)
}

function handleVisibleChange(e: any) {
    console.log(e)
    showAddSchemaDialog.value = e
}

function handlePluginVisibleChange(e: any) {
    showAddPlugin.value = e
}

function fetchNamespaceConfig() {
    var ns = route.query.ns as string
    globalStore.updateNamespaceTree()
    fetchConfig(ns).then(res => {
        conf.value = res.data
    }).catch(ex => {
        console.log(ex)
    })
}

function onAddStoreObject() {
    showAddSchemaDialog.value = true
}

function onAddPlugin() {
    showAddPlugin.value = true
}

function onAddQueryService() {
    openTypicalDraw("query", { params: [] })
}

function onReloadService() {
    fetchNamespaceConfig()
}

function handleOnUpload(resp) {
    console.log(resp)
    if (resp.status === 200 || resp.status === 0) {
        ElMessage.success("文件上传成功，并重新加载。")
    } else {
        let msg = resp.message
        if (msg.indexOf('Target file exist') > 0) {
            ElMessage.warning("文件上传成功，但归档文件已经存在，如果你确定要覆盖原有文件，请打开‘强制替换已存在的配置’选项。")
        } else {
            ElMessage.warning("上传处理失败，" + msg)
        }
    }
}

function handleDrawerClosed(e: any) {
    showDrawer.value = e
}

function get_auth_header() {
    return {
    "Authorization": "Bearer " + globalStore.token
    }
}

function openTypicalDraw(name: string, raw: any) {
    var t_name = name
    if (name === 'plugin') {
        if (raw.protocol === 'compose' 
                || raw.protocol === 'restapi' 
                || raw.protocol === 'kafka' 
                || raw.protocol === 'mqtt'
                || raw.protocol === 'elasticsearch'
                || raw.protocol === 'synctask') {
            t_name = raw.protocol
        }
    }
    show_panel.value = t_name
    select_object.value = raw
    showDrawer.value = true
    console.log('show ', name, raw)
} 

function onSaveConfig() {
    // save current
    var ns = route.query.ns as string
    console.log('Config: ' + JSON.stringify(conf.value))
    update(ns, 'config', conf.value).then(res => {
        if(res.status === 0 ||  res.status === 200) { 
            console.log('Save Success')
            globalStore.updateNamespaceTree()
            ElMessage.success('保存成功！');
        } else {
            ElMessage.error('保存失败！' + res.message);
        }
    }).catch((ex: any) => {
        console.log("exception ", ex)
        ElMessage.error('保存失败！' + ex);
    })
}

function onDeleteConfig() {
    var ns = route.query.ns as string
    configDelete(ns).then(res => {
        ElMessage.success('删除成功！');
        globalStore.updateNamespaceTree()
        globalStore.removeTabs(route.fullPath, true)
    })
}

function onArchiveConfig() {
    var ns = route.query.ns as string
    archive(ns).then(res => {
        console.log('Save Success')
        ElMessage.success('归档成功！');
        let part = res as any;
        var blob = new Blob([part], {
            type: 'application/zip;charset=utf-8'
        })
        const a = document.createElement('a')
        const URL = window.URL || window.webkitURL
        const href = URL.createObjectURL(blob)
        a.href = href
        a.download = ns + '.zip'
        document.body.appendChild(a)
        a.click()
        document.body.removeChild(a)
        URL.revokeObjectURL(href)
    }).catch((ex: any) => {
        console.log("exception ", ex)
        ElMessage.error('归档失败！' + ex);
    })
}

onMounted(() => {
    console.log("home");
    fetchNamespaceConfig();
});

onActivated(() => {
    console.log("activate")
    fetchNamespaceConfig();
})

onUpdated(() => {
    console.log("onUpdate")
    // fetchNamespaceConfig();
})
</script>

<style lang="scss" scoped>
@import "index.scss";
</style>
  