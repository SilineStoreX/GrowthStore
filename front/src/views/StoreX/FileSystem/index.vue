<template>
    <div class="home">
      <el-dialog
        v-model="showMovementForm"
        title="移动"
        width="800"
        align-center
        @close="onDialogClosed"
      >
      <el-form :model="moveform" label-width="100px" :inline="true" style="max-width: 600px">
          <el-form-item label="当前目录">
              <el-input v-model="moveform.path" style="width: 600px" placeholder="被移动的文件">
              </el-input>
          </el-form-item>
          <el-form-item label="移动到">
              <el-tree-select v-model="moveform.sub_path" lazy check-strictly :props="props" :data="selectfileroot" :load="fetchFolderTree" style="width: 600px"  />
          </el-form-item>
      </el-form>
        <template #footer>
          <div class="dialog-footer">
            <el-button @click="onDialogClosed">取消</el-button>
            <el-button type="primary" @click="onMovement">
              确认
            </el-button>
          </div>
        </template>
      </el-dialog>
      <el-dialog
        v-model="showUploadForm"
        title="上传文件"
        width="800"
        align-center
        @close="onDialogClosed"
      >
      <el-form :model="mkdirform" label-width="100px" :inline="true" style="max-width: 600px">
          <el-form-item label="当前目录">
              <el-input v-model="mkdirform.sub_path" style="width: 600px" placeholder="输入子目录名称用于创建">
                <template #prepend>
                  {{ current_path.path }}{{ current_path.path !== '/' ? '/' : '' }}
                </template>
                <template #append>
                  <el-button @click="doCreateSubPath">创建目录</el-button>
                </template>
              </el-input>
          </el-form-item>
          <el-upload
            class="upload-demo"
            :action="upload_file_uri"
            :headers="get_auth_header()"
            :on-preview="handlePreview"
            :on-remove="handleRemove"
            :before-remove="beforeRemove"
            :on-success="handleFileUploaded"
            multiple
            :limit="3"
            :on-exceed="handleExceed"
            :file-list="uploadFileList"
          >
            <el-button size="small" type="primary">点击上传</el-button>
            <template #tip>
              <div class="el-upload__tip">只能上传 jpg/png 文件，且不超过 500kb</div>
            </template>
          </el-upload>
      </el-form>
      </el-dialog>
      <el-dialog
        v-model="showEditForm"
        title="查看/编辑文件"
        width="800"
        align-center
        @close="onDialogClosed"
      >
      <el-tabs v-model="activateTabs" style="width: 100%">
          <el-tab-pane label="基本信息" name="basic">
            <el-form :model="hook" label-width="100px" :inline="true" style="max-width: 600px">
              <el-form-item v-if="!hook.isfolder" label="文件名称">
                  <el-input v-model="hook.filename" :rows="10" :readonly="!updatingName" style="width: 600px">
                    <template #append>
                      <el-button v-if="!updatingName" @click="modifyName">改名</el-button>
                      <el-button v-else @click="updateName">保存</el-button>
                    </template>
                  </el-input>
              </el-form-item>
              <el-form-item v-if="hook.isfolder" label="目录名称">
                  <el-input v-model="hook.filename" :rows="10" readonly style="width: 600px">
                    
                  </el-input>
              </el-form-item>
              <el-form-item label="路径">
                  <el-input v-model="hook.path" :rows="10" readonly style="width: 600px">
                    <template #append>
                      <el-button @click="movementFile">移动</el-button>
                    </template>
                  </el-input>
              </el-form-item>
              <el-form-item v-if="!hook.isfolder" label="大小">
                  <el-input v-model="hook.size" :rows="10" readonly style="width: 600px" />
              </el-form-item>
              <el-form-item label="最后访问">
                  <el-input v-model="hook.access_time" readonly :rows="10" style="width: 600px" />
              </el-form-item>
              <el-form-item label="修改时间">
                  <el-input v-model="hook.modify_time" readonly :rows="10" style="width: 600px" />
              </el-form-item>
              <el-form-item label="创建时间">
                  <el-input v-model="hook.create_time" readonly :rows="10" style="width: 600px" />
              </el-form-item>
              <el-form-item label="只读访问">
                  <el-switch v-model="hook.readonly" disabled :rows="10" style="width: 600px" />
              </el-form-item>
              <el-form-item v-if="!hook.isfolder" label="哈希值">
                  <el-input v-model="hook.hash" readonly :rows="10" style="width: 600px" />
              </el-form-item>
            </el-form>
          </el-tab-pane>
          <el-tab-pane v-if="!hook.isfolder" label="原始文件" name="raw">
            <el-image v-if="false && hook.content_type && hook.content_type.startsWith('image/')" fit="cover" :src="hook.download_url"></el-image>
            <ImageMarker v-if="hook.download_url && hook.content_type && hook.content_type.startsWith('image/')"
              :img-path="hook.download_url + '&_token=' + get_auth_token()"
              :rect-list.sync="rectList"
              :recoginzing-rect.sync="recoginzing_rect"
              :selected-rect-index.sync="selectedRectIndex"
              :original-rect.sync="image_natural_rect"
              :minimum-size="[10, 10]"
              :crop-image="doing_crop"
              @onCropImagedRect="onImageCorpped"
              @onSelectedRectIndex="onSelectedRectIndex"
            />
            <iframe v-else-if="hook.content_type && hook.content_type.startsWith('text/')" fit="cover" :src="hook.download_url + '&_token=' + get_auth_token()" style="width: 100%; height: 600px;"></iframe>
            <iframe v-else-if="hook.content_type && hook.content_type.endsWith('/pdf')" fit="cover" :src="hook.download_url + '&_token=' + get_auth_token()" style="width: 100%; height: 600px;"></iframe>
            <el-link v-else type="primary" :href="hook.download_url + '&_token=' + get_auth_token()">下载{{ hook.filename }}</el-link>
          </el-tab-pane>
          <el-tab-pane v-if="!hook.isfolder" label="文件内容" name="content">
            <el-form :model="hook" label-width="0px" :inline="false" label-position="top" style="max-width: 800px">
              <el-form-item label="以下为该文件的文字内容，它可能是通过对原始解码或识别所产生，也可以是由用户根据实际内容填写，同时这份内容将被用于AI索引，如果需要校正内容，请直接修改，然后保存，后台会对其进行重新索引。">
                  <el-input type="textarea" v-model="hook.content" :rows="10" style="width: 700px" />
              </el-form-item>
            </el-form>
          </el-tab-pane>
        </el-tabs>
        <template #footer>
          <div class="dialog-footer">
            <el-button @click="onDialogClosed">取消</el-button>
            <el-button type="primary" @click="onConfirm">
              确认
            </el-button>
          </div>
        </template>
      </el-dialog>
      <PageSplit :distribute="0.15" :lineThickness="6" :isVertical="true" @resizeLineStartMove="onresizeLineStartMove" @resizeLineMove="onResizeLineMove" @resizeLineEndMove="onresizeLineEndMove">
        <template v-slot:first>
            <el-scrollbar>
              <el-form label-width="80px" :inline="true">
                <el-form-item label="扩展名称">
                    <el-select v-model="select_plugin" style="width: 200px" @change="onQueryKeys">
                      <el-option v-for="it in pluginnames" :key="it" :value="it">{{ it }}</el-option>
                    </el-select>
                </el-form-item>
              </el-form>
              <el-tree
                style="max-height: calc(100vh);"
                :data="fileroots"
                :highlight-current="true"
                :props="props"
                :item-size="32"
                :height="treeViewHeight"
                lazy
                :load="fetchFolderTree"
                @current-change="nodeChanged"
                @node-expand="nodeExpanded"
              >
                <template #default="{node, data}">
                  <el-icon>
                    <component :is="data.icon" />
                  </el-icon>
                  <span>{{ data.label }}</span>
                </template>
              </el-tree>
            </el-scrollbar>
        </template>
        <template v-slot:second>
          <div class="folder-container">
            <div class="searchview">
              <div class="filesystem">
                <span class="title">文件系统</span>
                <div><span style="cursor: pointer;" data-path="/" @click="doSpanItemClick">/{{select_rootnode}} </span></div>
                <el-breadcrumb separator="/">
                  <el-breadcrumb-item v-for="item in path_items" :key="item.path"><span style="cursor: pointer;" :data-path="item.path" @click="doSpanItemClick">{{ item.path_name }}</span></el-breadcrumb-item>
                </el-breadcrumb>
              </div>
              <div class="button_func">
                <div class="percentbar">
                  <span>索引完成率</span>
                  <el-progress
                    :text-inside="true"
                    :stroke-width="24"
                    :percentage="percentage.percent"
                    style="width: 120px;"
                    status="success"
                  />
                </div>
                <div class="percentbar">
                  <el-button type="primary" @click="onUploadFile">创建文件</el-button>
                </div>
              </div>
            </div>
            <el-table :data="fileList" ref="redistable" :expand-row-keys="expandedkeys" row-key="uuid">
                <el-table-column type="index" label="序号" width="60px"/>
                <el-table-column prop="filename" label="名称" min-width="240px">
                  <template #default="scoped">
                    <span @click="doFileItemClick(scoped.row)" style="cursor: pointer;">
                      <el-icon>
                        <component :is="get_filetype(scoped.row.ext_name)" />
                      </el-icon>
                      {{ scoped.row.filename }}
                    </span>
                  </template>
                </el-table-column>
                <el-table-column prop="ext_name" label="类型" width="120px">
                  <template #default="scoped">
                    {{ scoped.row.isfolder ? '目录' : scoped.row.ext_name }}
                  </template>
                </el-table-column>
                <el-table-column prop="create_time" label="创建日期" width="180px">
                  <template #default="scoped">
                    {{ scoped.row.create_time ? new Date(scoped.row.create_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                  </template> 
                </el-table-column>
                <el-table-column prop="modify_time" label="修改日期" width="180px">
                  <template #default="scoped">
                    {{ scoped.row.modify_time ? new Date(scoped.row.modify_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                  </template> 
                </el-table-column>
                <el-table-column prop="size" label="大小" width="120px">
                  <template #default="scoped">
                    {{ !scoped.row.isfolder ? formatHumanSize(scoped.row.size) : '' }}
                  </template>
                </el-table-column>
                <el-table-column prop="readonly" label="权限" width="80px">
                  <template #default="scoped">
                    {{ scoped.row.readonly ? '只读' : '' }}
                  </template>
                </el-table-column>
                <el-table-column label="操作" width="100px">
                    <template #default="scoped">
                      <el-button type="primary" icon="SetUp" circle @click="onEditKey(scoped.row)"/>
                      <el-popconfirm title="确认要删除吗?" @confirm="onDelKey(scoped.row)">
                          <template #reference>
                              <el-button type="danger" icon="Delete" circle />
                          </template>
                      </el-popconfirm>
                    </template>
                </el-table-column>
              </el-table>
            </div>
        </template>
      </PageSplit>
    </div>
  </template>
  
  <script lang="ts" setup name="filesystem">
  import { computed, onMounted, ref } from "vue";
  import { useRoute } from "vue-router";
  import { GlobalStore } from "@/stores";
  import ImageMarker from '@/components/ImageMarker/ImageMarker.vue'
  import { filesystem_root, filesystem_tree, filesystem_list, filesystem_delete, filesystem_info, filesystem_rec, filesystem_percent, filesystem_mkdir, filesystem_update, filesystem_move } from "@/http/modules/filesystem";
  import { fetch_pluginnames } from "@/http/modules/management";
  import { ElMessage } from "element-plus";
  import PageSplit from "vue3-page-split";
  import "vue3-page-split/dist/style.css";
  import type Node from 'element-plus/es/components/tree/src/model/node'
  import '@/utils/datetime.js'
  import { formatHumanSize } from '@/utils/utils'
  import { get_filetype } from '@/utils/filetype'

  const globalStore = GlobalStore();
  const route = useRoute()
  const fileList = ref<Array<any>>([])
  const uploadFileList = ref<Array<any>>([])
  const pluginnames = ref<Array<any>>([])
  const select_plugin = ref<string>("")
  const cmdfrm = ref<any>({})
  const path_items = ref<Array<any>>([])
  const expandedkeys = ref<Array<any>>([])
  const hook = ref<any>({})
  const moveform = ref<any>({})
  const updateIndexMode = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)
  const showMovementForm = ref<boolean>(false)
  const showUploadForm = ref<boolean>(false)
  const computedContainer = ref(null);
  const treeViewHeight = ref(1024);
  const select_rootnode = ref(null)
  const current_path = ref<any>({

  })
  const mkdirform = ref<any>({})
  const activateTabs = ref('basic')
  const percentage = ref({
    percent: 0.0
  })
  const updatingName = ref<boolean>(false)

  const rectList = ref<Array<any>>([])
  const recoginzing_rect = ref<Array<any>>([])
  const image_natural_rect = ref<any>({ left: 0, top: 0, width: 0, height: 0 })
  const selectedRectIndex = ref(0)
  const doing_crop = ref(false)

  const upload_file_uri = computed(() => {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    var currpath = current_path.value.path
    return `/api/filesystem/${ns}/${name}/${rmth}/upload?path=${currpath}`
  })

  let observer = null;

  interface Tree {
    id: string
    label: string
    leaf?: boolean
    children?: Tree[]
  }

  const fileroots = ref<Tree[]>([])

  const selectfileroot = ref<Array<any>>([])

  function onImageCorpped(rt, sz) {
  }

  function onSelectedRectIndex(i) {
  }

  function movementFile() {
    showEditForm.value = false;
    moveform.value = {
      path: hook.value.path,
      sub_path: ''
    }

    var rnode = select_rootnode.value
    

    setTimeout(()=> {
      selectfileroot.value = fileroots.value.filter((p) => p.label == rnode)
      showMovementForm.value = true;
    }, 100)
  }

  function modifyName() {
    updatingName.value = true
  }

  function updateName() {
    updatingName.value = false
    file_metadata.value = {
      ...file_metadata.value,
      filename: hook.value.filename
    }
  }

  function handleRemove(file, fileList) {
    console.log(file, fileList)
  }

  function handlePreview(file) {
        console.log(file)
  }

  function handleFileUploaded(e) {
    console.log('FileUploaded', e)
    doFileItemClick({ path: current_path.value.path, isfolder: true });
  }

  function get_auth_header() {
    if (globalStore.api_token) {
      return {
        "Authorization": "Bearer " + get_auth_token()
      }
    } else if (globalStore.token) {
      return {
        "Authorization": "Bearer " + globalStore.token
      }
    } else {
      return {}
    }
  }

  function get_auth_token() {
    return globalStore.api_token
  }

  function handleExceed(files, fileList) {
      ElMessage.warning(
        `当前限制选择 3 个文件，本次选择了 ${files.length} 个文件，共选择了 ${
          files.length + fileList.length
        } 个文件`
      )
  }

  function beforeRemove(file, fileList) {
    ElMessage(`确定移除 ${file.name}？`)
    return true
  }
  
  const handleResize = () => {
    if (computedContainer.value && computedContainer.value.$el) {
      // const width = computedContainer.value.$el.offsetWidth;
      const height = computedContainer.value.$el.offsetHeight;
      // console.log(`Size: width=${width}, height=${height}`);
      treeViewHeight.value = height - 40;
    }
  };

  function onresizeLineStartMove() {
  console.log("onresizeLineStartMove");
}

function onResizeLineMove(e: any) {
  console.log("onResizeLineMove :>> ", e);
}

  function onresizeLineEndMove() {
    console.log("onresizeLineEndMove");
  }

  const props = {
    value: 'id',
    label: 'label',
    icon: 'icon',
    isLeaf: 'isfolder',
    children: 'children',
  }

  function onDelKeys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    onDelKey(key)
  }

  function fetchPluginNames() {
    var ns = route.query.ns as string
    fetch_pluginnames(ns, "filesystem").then(res => {
      pluginnames.value = res.data
      if (res.data.length >= 1) {
        select_plugin.value = res.data[0]
      }
      fetch_indexes()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function splite_path(cpath) {
    var path = cpath === '' ? '/': cpath
    let pt = path.split("/");
    var ntp = []
    var rmth = select_rootnode.value
    for(var t in pt) {
      var ntt = [...pt].splice(0, parseInt(t) + 1); //pt.slice(0, t + 1)
      var pathspt = pt[t]
      var cp = {
        path_name: pathspt,
        path: ntt.join("/")
      }
      console.log(t + '     ss: ' + JSON.stringify(cp))
      ntp.push(cp)
    }
    return ntp;
  }

  function get_plugin_name() {
    let name = select_plugin.value
    if (name && name !== '') {
      return name
    } else {
      return route.query.name as string
    }
  }

  function onMovement() {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    var movedata = {
      ...moveform.value
    }

    if (movedata.sub_path.startsWith('_root_id:')) {
      movedata.sub_path = "/"
    }

    console.log("Movement", moveform.value)
    filesystem_move(ns, name, rmth, movedata).then(res => {
      if (res.status === 200 || res.status === 0) {
        doFileItemClick({path: current_path.value.path, isfolder: true})
        showMovementForm.value = false
      } else {
        ElMessage.warning("请求文件移动失败，原因：" + res.message)
      }
    }).catch(ex => {
      ElMessage.warning("请求文件移动失败。" + ex.to_string())
    })
  }

  function onUploadFile() {
    updateIndexMode.value = false
    let jsonbody = {
      aliases: {},
      mappings: {},
      settings: {}
    }
    hook.value = {jsonbody: JSON.stringify(jsonbody)}
    showUploadForm.value = true
  }

  function onDialogClosed() {
    updateIndexMode.value = false
    showEditForm.value = false
    showUploadForm.value = false
    showMovementForm.value = false
    mkdirform.value = {}
    // moveform.value = {}
  }

  const file_metadata = ref<any>({})

  function onEditKey(row) {
    updateIndexMode.value = true
    hook.value = {
      ...row
    }
    activateTabs.value = 'basic'
    showEditForm.value = true
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    selectfileroot.value = fileroots.value.filter((p) => p.label == rmth)
    filesystem_info(ns, name, rmth, { path: row.path }).then(res => {
      if (res.status === 200 && res.data) {
        var dt = res.data
        if (res.data.metadata) {
          var mt = { ...res.data.metadata }
          file_metadata.value = mt
          var nt = res.data
          nt.metadata = undefined
          dt = { ...mt, ...nt }
        }
        console.log('Donwload: ', dt)
        dt.download_url = `/api/filesystem/${ns}/${name}/${rmth}/get?path=` + dt.path
        hook.value = dt
      } 
    })

    fileregconize(row)
  }

  function fileregconize(row) {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    selectfileroot.value = fileroots.value.filter((p) => p.label == rmth)
    filesystem_rec(ns, name, rmth, { path: row.path }).then(res => {
      if (res.status === 200 && res.data) {
        var dt = res.data
        console.log('Rec respose: ', dt)
      } 
    })
  }

  function nodeChanged(data: any, node: Node) {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    console.log('nodeChanged', node)
    var rtmethod = get_root_node(node);
    select_rootnode.value = rtmethod
    const row = data.treedata
    filesystem_list(ns, name, rtmethod, row).then(res => {
      fileList.value = res.data
      current_path.value = row
      console.log('Current Path: ', current_path.value) 
      path_items.value = splite_path(row.path)
    }).catch(ex => {
      console.log(ex)
    })

    fetch_percentage(node)
  }

  function nodeExpanded(node: any) {

  }

  function get_root_node(nd: Node) {
    if (nd.level === 1) {
      return nd.label
    } else {
      if (nd.parent) {
        return get_root_node(nd.parent)
      } else {
        return null
      }
    }
  }

  function fetch_percentage(node) {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rtmethod = get_root_node(node);    
    filesystem_percent(ns, name, rtmethod, {}).then(res => {
      if (res.status === 200) {
        var pt = res.data
        var perct = pt && pt.total === 0 ? '0.0': (pt.processed / pt.total * 100.0).toFixed(1)
        percentage.value = {
          ...pt,
          percent: parseFloat(perct)
        }
      }
    }).catch(ex => {
      percentage.value = {
        percent: 0.0
      }
    })
  }

  const fetchFolderTree = (node: Node, resolve: (data: Tree[]) => void, reject: () => void) => {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    if (node === null) {
      if (reject) {
        reject();
      }
      return;
    }

    console.log('Tree Node: ', node)
    var rtmethod = get_root_node(node)

    if (!rtmethod || rtmethod === '')  {
      if (reject) {
        reject();
      }
      return;
    }

    var data = {
      path: node.data?.treedata?.path,
      operator: 'tree'
    }
    
    filesystem_tree(ns, name, rtmethod, data).then(res => {
      let subs = res.data.map(t => {
        return {
          id: t.path,
          label: t.filename,
          icon: 'Folder',
          treedata: t,
        }
      })
      resolve(subs)
    }).catch(ex => {
      if (reject) {
        reject()
      }
    })
  }

  function doFileItemClick(row) {
    console.log('Row: ', row)
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    if (row.isfolder) {
      filesystem_list(ns, name, rmth, row).then(res => {
        fileList.value = res.data
        current_path.value = row
        path_items.value = splite_path(row.path)
      }).catch(ex => {
        console.log(ex)
      })
    }
  }

  function doSpanItemClick(er) {
    var path = er.target.attributes['data-path'].value
    console.log(er.target.attributes['data-path'].value)
    doFileItemClick({path, isfolder: true})
  }

  function doCreateSubPath() {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value

    filesystem_mkdir(ns, name, rmth, {
      path: current_path.value.path,
      sub_path: mkdirform.value.sub_path,      
    }).then(res => {
      console.log(res)
      if (res.status === 200) {
        mkdirform.value = { ...mkdirform.value, sub_path: ''}
        let newpath = res.data.path
        doFileItemClick({
          path: newpath,
          isfolder: true
        })
      }
    })
  }

  function onConfirm() {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value

    let fmt = file_metadata.value
    fmt.content = hook.value.content
    
    if (hook.value.filename !== fmt.filename) {
      fmt.filename = hook.value.filename
    }

    let fsr = {
      path: hook.value.path,
      metadata: fmt,
      operator: 'update'
    }

    
    filesystem_update(ns, name, rmth, fsr).then(res => {
      if (res.status === 200 || res.status === 0) {
        showEditForm.value = false
        doFileItemClick({path: current_path.value.path, isfolder: true})
      } else {
        ElMessage.warning("保存失败：" + res.message)
      }
    }).catch(ex => {
      console.log(ex)
      ElMessage.warning("保存失败：" + ex.message)
    })
  }

  function onDelKey(row) {
    var ns = route.query.ns as string
    var name = get_plugin_name()
    var rmth = select_rootnode.value
    let fulpath = row.path
    filesystem_delete(ns, name, rmth, {
      path: fulpath
    }).then(res => {
      let pt = fulpath.substring(0, fulpath.lastIndexOf("/"))
      console.log(pt)
      filesystem_list(ns, name, rmth, { path: pt }).then(res => {
        fileList.value = res.data
      }).catch(ex => {
        console.log(ex)
      })
    }).catch(ex => {
      console.log(ex)
    })
  }

  function fetch_indexes() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    var method = "list" + (key === '' ? '' : ':' + key)
    filesystem_root(ns, get_plugin_name()).then(res => {
      fileroots.value = res.data.map(t => {
        let r = {
          id: '_root_id:' + t.filename + t.path,
          label: t.filename,
          icon: 'Folder',
          leaf: false,
          treedata: t,
          chidren: []
        }
        return r;
      })

      selectfileroot.value = fileroots.value
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onQueryKeys() {
    fetch_indexes()
  }

  onMounted(() => {
    console.log("filesystem");
    fetchPluginNames()
    fetch_indexes();
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  