<template>
    <div class="home">
      <el-row :gutter="20">
        <el-col :span="6">
          <div class="kbheader">
            <span class="text-large font-600 mr-3">知识库<el-button type="primary" icon="Plus" text @click="onAddKnowledge">新建</el-button></span>
            <el-divider />
          </div>
          <el-scrollbar :height="scrollHeight">
            <el-card v-for="kb in knowledges" :key="kb.id" shadow="hover" :class="'kbcard ' + (current_knowledge.id === kb.id ? 'selected' : '')">
              <div class="kb-container" @click="onSelectKnowledge(kb)">
                <el-avatar :size="50" :src="kb.avatar + '?_token=' + get_auth_token()" /> <span>{{ kb.name }}</span>
              </div>
              <div class="kb-action">
                <div class="summary">
                  <el-tooltip v-if="kb.docs_num" class="box-item" effect="dark" content="文档数量" placement="top">
                      <span><el-icon><Document /></el-icon>{{ kb.docs_num }}</span>
                  </el-tooltip>
                  <el-tooltip v-if="kb.chunk_num" class="box-item" effect="dark" content="文本块数量" placement="top">
                      <span><el-icon><ChatLineSquare /></el-icon>{{ kb.chunk_num }}</span>
                  </el-tooltip>
                  <el-tooltip v-if="kb.token_num" class="box-item" effect="dark" content="Token数量" placement="top">
                      <span><el-icon><Reading /></el-icon>{{ kb.token_num }}</span>
                  </el-tooltip>
                </div>
                <div class="actions">
                  <el-button icon="Edit" type="primary" text @click="onEditKnowledge(kb)">编辑</el-button>
                  <el-popconfirm title="确定要删除该知识库吗?" @confirm="onDelKnowledge(kb)">
                    <template #reference>
                        <el-button icon="Delete" type="danger" text>删除</el-button>
                    </template>
                  </el-popconfirm>
                </div>
              </div>
            </el-card>
            <el-card class="kbcard" shadow="hover">
              <div class="plus-container" @click="onAddKnowledge">
                <el-icon><Plus /></el-icon>
              </div>
            </el-card>
          </el-scrollbar>
        </el-col>
        <el-col :span="18">
          <el-header v-if="current_knowledge && current_knowledge.id" class="page_header">
            <div class="topbar">
              <div class="title">{{ current_knowledge.name }}</div>
              <div class="action">
                <el-button type="primary" icon="Plus" text @click="onAddDocument">新建文档</el-button>
                <el-button type="primary" icon="Plus" text @click="onAddStructDocument">新建结构化</el-button>
                <el-button type="success" icon="Plus" text @click="onAddChunkDirect">新建块</el-button>
              </div>
            </div>
            <el-divider />
            <div class="topbar">
              <div class="title" style="display: inline-block; width: 900px">
                <el-input type="textarea" v-model="queryfrm.question" :rows="2" style="min-width: 600px;" placeholder="输入文本内容，用于召回测试"/>
                <el-form-item label="召回参数" style="padding-top: 5px;">
                  <el-form-item label="相似度大于"><el-input v-model="queryfrm.score_threshold" /></el-form-item>
                  <el-form-item label="Top-K"><el-input v-model="queryfrm.top_k" /></el-form-item>
                </el-form-item>
              </div>
              <div class="action">
                <el-button type="success" icon="Search" @click="onRetrievalDocument">召回测试</el-button>
              </div>
            </div>
          </el-header>
          <el-tabs v-if="current_knowledge && current_knowledge.id" v-model="activateDocsTabs" type="border-card" style="width: 100%">
            <el-tab-pane label="文档" name="docset">
              <el-table :data="documentsets" v-loading="showLoading" ref="redistable" :height="scrollHeight - 200">
                <el-table-column type="index" width="60" label="序号" />
                <el-table-column prop="name" label="名称"  show-overflow-tooltip />
                <el-table-column prop="token_num" label="Token数量" width="100px" />
                <el-table-column prop="chunk_num" label="块数量" width="100px" />
                <el-table-column prop="status" label="状态" width="100px">
                  <template #default="scoped">
                    <el-tag v-if="scoped.row.status === 0" type="success">成功</el-tag>
                    <el-tag v-else-if="scoped.row.status === 1" type="primary">等待</el-tag>
                    <el-tag v-else-if="scoped.row.status === 2" type="warning">处理中</el-tag>
                    <el-tag v-else-if="scoped.row.status === 5" type="danger">失败</el-tag>
                    <el-tag v-else type="info">未处理</el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="enabled" label="有效" width="100px">
                  <template #default="scoped">
                    {{ scoped.row.enabled ? '有效': '无效' }}
                  </template>
                </el-table-column>
                <el-table-column prop="update_time" label="更新时间" width="220px">
                  <template #default="scoped">
                    {{ scoped.row.update_time ? new Date(scoped.row.update_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                  </template>
                </el-table-column>
                <el-table-column label="操作" width="100px">
                    <template #default="scoped">
                      <el-button type="primary" icon="SetUp" circle @click="onEditDocset(scoped.row)"/>
                      <el-popconfirm title="确认要删除吗?" @confirm="onDelDocset(scoped.row.id)">
                          <template #reference>
                              <el-button type="danger" icon="Delete" circle />
                          </template>
                      </el-popconfirm>
                    </template>
                </el-table-column>
              </el-table>
              <div class="pager-container">
                  <el-pagination :current-page="DocsPageCurrent"
                                :page-size="DocsPageSize" 
                                background layout="total, sizes, prev, pager, next"
                                :page-sizes="[20, 50, 100, 200, 300, 400]" :total="DocsPageTotal" 
                                @size-change="handleDocsSizeChange"
                                @current-change="handleDocsCurrentChange"/>
              </div>
            </el-tab-pane>
            <el-tab-pane label="结构化文档" name="structureset">
              <el-text type="primary">结构化文档是将结构化的记录转换成文本块的方式。可以将数据库记录、外部系统的查询等转换成文本块，生成Embedding，并加入到知识库。</el-text>
              <el-table :data="structsets" v-loading="showLoading" ref="redistable" :height="scrollHeight - 230">
                <el-table-column type="index" width="60" label="序号" />
                <el-table-column prop="invoke_uri" label="Invoke URI" show-overflow-tooltip  />
                <el-table-column prop="description" label="描述" show-overflow-tooltip  />
                <el-table-column prop="token_num" label="Token数量" width="100px" />
                <el-table-column prop="chunk_num" label="块数量" width="100px" />
                <el-table-column prop="status" label="状态" width="80px">
                  <template #default="scoped">
                    <el-tag v-if="scoped.row.status === 0" type="success">成功</el-tag>
                    <el-tag v-else-if="scoped.row.status === 1" type="primary">等待</el-tag>
                    <el-tag v-else-if="scoped.row.status === 2" type="warning">处理中</el-tag>
                    <el-tag v-else-if="scoped.row.status === 5" type="danger">失败</el-tag>
                    <el-tag v-else type="info">未处理</el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="enabled" label="有效" width="80px">
                  <template #default="scoped">
                    {{ scoped.row.enabled ? '有效': '无效' }}
                  </template>
                </el-table-column>
                <el-table-column prop="update_time" label="更新时间" width="200px">
                  <template #default="scoped">
                    {{ scoped.row.update_time ? new Date(scoped.row.update_time).strftime('%Y-%m-%d %H:%M:%S') : '' }}
                  </template>
                </el-table-column>
                <el-table-column label="操作" width="100px">
                    <template #default="scoped">
                      <el-button type="primary" icon="SetUp" circle @click="onEditStruct(scoped.row)"/>
                      <el-popconfirm title="确认要删除吗?" @confirm="onDelStruct(scoped.row)">
                          <template #reference>
                              <el-button type="danger" icon="Delete" circle />
                          </template>
                      </el-popconfirm>
                    </template>
                </el-table-column>
              </el-table>
              <div class="pager-container">
                  <el-pagination :current-page="StructPageCurrent"
                                :page-size="StructPageSize" 
                                background layout="total, sizes, prev, pager, next"
                                :page-sizes="[20, 50, 100, 200, 300, 400]" :total="StructPageTotal" 
                                @size-change="handleStructSizeChange"
                                @current-change="handleStructCurrentChange"/>
              </div>
            </el-tab-pane>            
            <el-tab-pane label="文本块" name="chunkset">
              <el-table :data="chunksets" v-loading="showLoading" ref="redistable" :height="scrollHeight - 200">
                <el-table-column type="index" width="60" label="序号" />
                <el-table-column prop="chunk_no" label="块号" width="90px" />
                <el-table-column prop="chunk_type" label="类型" width="100px" />
                <el-table-column prop="token_num" label="Token数量" width="100px" />
                <el-table-column prop="chunk_text" label="块文本" show-overflow-tooltip />
                <el-table-column prop="chunk_status" label="状态" width="100px">
                  <template #default="scoped">
                    <el-tag v-if="scoped.row.chunk_status === 0" type="success">成功</el-tag>
                    <el-tag v-else-if="scoped.row.chunk_status === 1" type="primary">等待</el-tag>
                    <el-tag v-else-if="scoped.row.chunk_status === 2" type="warning">处理中</el-tag>
                    <el-tag v-else-if="scoped.row.chunk_status === 5" type="danger">失败</el-tag>
                    <el-tag v-else type="info">未处理</el-tag>
                  </template>
                </el-table-column>
                <el-table-column v-if="showSimilarity" prop="similarity" label="相似度" width="120px">
                  <template #default="scoped">
                    {{ scoped.row.similarity ? scoped.row.similarity.toFixed(5): '' }}
                  </template>          
                </el-table-column>                
                <el-table-column v-else prop="update_time" label="更新时间" width="200px">
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
            </el-tab-pane>
            <el-tab-pane label="召回测试" name="retrieval">
              <el-table :data="retrievals" v-loading="showLoading" :height="scrollHeight - 160">
                <el-table-column type="index" width="60" label="序号" />
                <el-table-column prop="doc_id" label="文档ID" width="120px" show-overflow-tooltip />
                <el-table-column prop="doc_title" label="文档标题" width="200px" show-overflow-tooltip />
                <el-table-column prop="content" label="块文本" show-overflow-tooltip />
                <el-table-column prop="similarity" label="相似度" width="120px">
                  <template #default="scoped">
                    {{ scoped.row.similarity ? scoped.row.similarity.toFixed(5): '' }}
                  </template>          
                </el-table-column>
              </el-table>
            </el-tab-pane>
          </el-tabs>
        </el-col>
      </el-row>
      <variable-management :show="showVariableDialog" @update:show="handleVariableUpdate"/>
      <el-drawer title="编辑知识库" v-model="showEditForm" :with-header="true" :size="900">
        <el-form v-model="editfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="名称">
            <el-input v-model="editfrm.name" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="语言">
            <el-input v-model="editfrm.language" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="头像">
              <el-upload
                  class="upload-demo"
                  :headers="get_auth_header()"
                  :action="upload_file_uri"
                  :show-file-list="false"
                  method="post"
                  accept="image/*"
                  :on-success="handleFileUploaded"
              >
                <el-avatar v-if="editfrm.avatar" :size="80" :src="editfrm.avatar + '?_token=' + get_auth_token()" />
                <el-icon v-else style="width: 80px; height:80px; border: 1px dashed #ccc; border-radius: 10px;"><Plus /></el-icon>
              </el-upload>
          </el-form-item>
          <el-form-item label="详细描述">
            <el-input v-model="editfrm.description" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
          <el-form-item label="相似度阀值">
            <el-input v-model="editfrm.similarity_threshold" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="相似度比重">
            <el-input v-model="editfrm.vector_similarity_weight" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="状态">
            <el-switch v-model="editfrm.status" style="width: 280px"></el-switch>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="primary" @click="onConfirm">
            <el-icon class="el-icon--left"><CircleCheckFilled /></el-icon>
            保存
          </el-button>
          <el-button type="danger" @click="onDialogClosed">
            <el-icon class="el-icon--left"><CircleCloseFilled /></el-icon>
            关闭
          </el-button>
        </template>
      </el-drawer>
      <el-drawer title="编辑文档" v-model="showDocumentEditForm" :with-header="true" :size="900">
        <el-form v-model="docfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item v-if="docfrm.id" label="文档ID">
            <el-input v-model="docfrm.id" style="width: 700px" disabled></el-input>
          </el-form-item>
          <el-form-item label="名称">
            <el-input v-model="docfrm.name" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item v-if="!(docfrm.id && docfrm.fullpath)" label="上传文档">
              <el-upload
                  class="upload-demo"
                  :headers="get_auth_header()"
                  :action="upload_file_uri"
                  :show-file-list="true"
                  :limit="1"
                  method="post"
                  accept="*/*"
                  :on-success="handleDocumentUploaded"
              >
                <el-button size="small" type="primary">点击上传</el-button>
                <template #tip>
                  <div class="el-upload__tip">请上传文档类型文件，如Word、PDF、图片等，且不超过100MB</div>
                </template>
              </el-upload>
          </el-form-item>
          <el-form-item label="文档">
            <el-input v-model="docfrm.fullpath" style="width: 700px" disabled></el-input>
          </el-form-item>
          <el-form-item label="详细描述">
            <el-input v-model="docfrm.description" type="textarea" style="width: 700px" :rows="6" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
          <el-form-item v-if="docfrm.process_config" label="提取与分块">
            <el-card style="width: 700px" shadow="never">
              <el-form-item label="分块方式">
                <el-radio-group v-model="docfrm.process_config.type" size="default">
                  <el-radio-button value="fixed">固定字数</el-radio-button>
                  <el-radio-button value="semantics">语义</el-radio-button>
                  <el-radio-button value="paragraph">自然段落</el-radio-button>
                </el-radio-group>
              </el-form-item>
              <el-form-item v-if="docfrm.process_config.type === 'fixed'" label="字数">
                <el-input v-model="docfrm.process_config.fixed_numb" style="width: 500px"></el-input>
              </el-form-item>
              <el-form-item v-if="docfrm.process_config.type === 'fixed' || docfrm.process_config.type === 'paragraph'" label="保留自然段落">
                <el-switch v-model="docfrm.process_config.keep_nature_paragraph" style="width: 500px" />
              </el-form-item>
              <el-form-item v-if="docfrm.process_config.type === 'fixed' || docfrm.process_config.type === 'paragraph'" label="忽略极小段落">
                <el-switch v-model="docfrm.process_config.ignore_short_paragraph" style="width: 500px" />
              </el-form-item>
            </el-card>
          </el-form-item>
          <el-form-item v-if="docfrm.id && docfrm.chunk_num" label="块数量">
            <el-text style="width: 280px">{{ docfrm.chunk_num }}</el-text>
          </el-form-item>
          <el-form-item v-if="docfrm.id && docfrm.token_num" label="Token数量">
            <el-text style="width: 280px">{{ docfrm.token_num }}</el-text>
          </el-form-item>
          <el-form-item label="状态">
            <el-tag v-if="docfrm.status === 0" type="success">成功</el-tag>
            <el-tag v-else-if="docfrm.status === 1" type="primary">等待</el-tag>
            <el-tag v-else-if="docfrm.status === 2" type="warning">处理中</el-tag>
            <el-tag v-else-if="docfrm.status === 5" type="danger">失败</el-tag>
            <el-tag v-else type="info">未处理</el-tag>
          </el-form-item>
          <el-form-item label="启用">
            <el-switch v-model="docfrm.enabled" style="width: 280px"></el-switch>
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
      <el-drawer title="编辑结构化文档" v-model="showStructEditForm" :with-header="true" :size="900">
        <el-form v-model="structfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="Invoke URI">
            <el-input v-model="structfrm.invoke_uri" style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="执行参数">
            <el-input v-model="structfrm.invoke_params" type="textarea" style="width: 700px" :rows="4" placeholder="以JSON格式表示的用于调用InvokeURI时的参数"></el-input>
            <el-text>参数模板中可以使用${variable.name}的方式引用参数名称中的变量。该变量用于增量转换处理。</el-text>
          </el-form-item>
          <el-form-item label="分页请求">
            <el-switch v-model="structfrm.paged_query" style="width: 700px"></el-switch>
          </el-form-item>
          <el-form-item label="ID字段">
            <el-input v-model="structfrm.id_field" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="引用字段">
            <el-input v-model="structfrm.variable_referer" style="width: 280px"></el-input>
          </el-form-item>
          <el-form-item label="变量名称">
            <el-input v-model="structfrm.variable_name" style="width: 280px">
              <template #append>
                <el-button icon="Operation" @click="handleVariableManage">变量管理</el-button>
              </template>
            </el-input>
          </el-form-item>
          <el-form-item label="变量类型">
            <el-radio-group v-model="structfrm.variable_type" size="default" style="width: 280px">
              <el-radio-button v-for="t in variable_types" :key="t" :value="t">
                {{ t }}
              </el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="详细描述">
            <el-input v-model="structfrm.description" type="textarea" style="width: 700px" :rows="3" placeholder="请详细准确的描述该提示词的用途，以便能够准确召回"></el-input>
          </el-form-item>
          <el-form-item label="文档化模板">
            <el-input v-model="structfrm.process_template" type="textarea" style="width: 700px" :rows="6" placeholder="请在此处输入文档化处理的模板，采用Tera模板格式标准。"></el-input>
            <el-text type="info">在文档格式化模板中，可以直接引用结构化中字段属性，其引用方式为record.name这种方式。</el-text>
            <el-text type="success">如果需要生成问答式的文本块，可以在模板中为每一个问答部分加入“---Q&A---”，以表示其以下为问答部分。</el-text>
          </el-form-item>
          <el-form-item v-if="structfrm.id && structfrm.chunk_num" label="块数量">
            <el-text v-model="structfrm.chunk_num" style="width: 280px"></el-text>
          </el-form-item>
          <el-form-item  v-if="structfrm.id && structfrm.token_num" label="Token数量">
            <el-text v-model="structfrm.token_num" style="width: 280px"></el-text>
          </el-form-item>
          <el-form-item  v-if="structfrm.id" label="状态">
            <el-tag v-if="structfrm.status === 0" type="success">成功</el-tag>
            <el-tag v-else-if="structfrm.status === 1" type="primary">等待</el-tag>
            <el-tag v-else-if="structfrm.status === 2" type="warning">处理中</el-tag>
            <el-tag v-else-if="structfrm.status === 5" type="danger">失败</el-tag>
            <el-tag v-else type="info">未处理</el-tag>
          </el-form-item>
          <el-form-item label="启用">
            <el-switch v-model="structfrm.enabled" style="width: 280px"></el-switch>
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button type="primary" @click="onConfirmStruct">
            <el-icon class="el-icon--left"><CircleCheckFilled /></el-icon>
            保存
          </el-button>
          <el-button type="danger" @click="onDialogClosed">
            <el-icon class="el-icon--left"><CircleCloseFilled /></el-icon>
            关闭
          </el-button>
        </template>
      </el-drawer>
      <el-drawer title="编辑文本块" v-model="showChunkEditForm" :with-header="true" :size="900">
        <el-form v-model="chunkfrm" v-loading="showLoading" label-width="100px" :inline="true">
          <el-form-item label="块类型">
            <el-radio-group v-model="chunkfrm.chunk_type" size="default">
              <el-radio-button value="normal">普通文本块</el-radio-button>
              <el-radio-button value="question">问答块</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="文本块">
            <el-input v-model="chunkfrm.chunk_text" type="textarea" style="width: 700px" :rows="6" placeholder="请填写文本块"></el-input>
          </el-form-item>
          <el-form-item label="Token数量">
            <el-input v-model="chunkfrm.token_num" disabled style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="文档ID">
            <el-input v-model="chunkfrm.doc_id" disabled style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="引用ID">
            <el-input v-model="chunkfrm.chunk_ref" disabled style="width: 700px"></el-input>
          </el-form-item>
          <el-form-item label="状态">
            <el-tag v-if="chunkfrm.chunk_status === 0" type="success">成功</el-tag>
            <el-tag v-else-if="chunkfrm.chunk_status === 1" type="primary">等待</el-tag>
            <el-tag v-else-if="chunkfrm.chunk_status === 2" type="warning">处理中</el-tag>
            <el-tag v-else-if="chunkfrm.chunk_status === 5" type="danger">失败</el-tag>
            <el-tag v-else type="info">未处理</el-tag>
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
  import { knowledge_query, knowledge_save, knowledge_delete, knowledge_get, chunkset_query, chunkset_get, chunkset_retrieval_test, chunkset_save, chunkset_delete, docset_save, docset_query, docset_delete } from "@/http/modules/growthai";
  import { structset_save, structset_query, structset_delete, structset_get } from "@/http/modules/growthai";
  import VariableManagement from "./variables.vue";
  import { ElMessage } from "element-plus";
  import { GlobalStore } from "@/stores";

  const globalStore = GlobalStore();
  const route = useRoute()
  const queryfrm = ref<any>({})
  const PageCurrent = ref<any>(1)
  const PageSize = ref<any>(20)
  const PageTotal = ref(0)
  const DocsPageCurrent = ref<any>(1)
  const DocsPageSize = ref<any>(20)
  const DocsPageTotal = ref(0)

  const StructPageCurrent = ref<any>(1)
  const StructPageSize = ref<any>(20)
  const StructPageTotal = ref(0)  

  const variable_types = ref<Array<any>>(["Number", "DateTime"])
  const activateDocsTabs = ref('docset')
  const current_knowledge = ref<any>({})
  const scrollHeight = ref<any>(600)
  const knowledges = ref<Array<any>>([])
  const chunksets = ref<Array<any>>([])
  const structsets = ref<Array<any>>([])
  const retrievals = ref<Array<any>>([])
  const documentsets = ref<Array<any>>([])
  const cmdfrm = ref<any>({})
  const docfrm = ref<any>({})
  const structfrm = ref<any>({})
  const showChunkEditForm = ref(false)
  const showDocumentEditForm = ref(false)
  const showStructEditForm = ref(false)
  const chunkfrm = ref<any>({})
  const editfrm = ref<any>({})
  const hook = ref<any>({})
  const showSimilarity = ref<boolean>(false)
  const showEditForm = ref<boolean>(false)
  const showLoading = ref<boolean>(false)
  const showVariableDialog = ref<boolean>(false)

  function get_auth_header() {
    return {
      "Authorization": "Bearer " + get_auth_token()
    }
  }

  function get_auth_token() {
    return globalStore.api_token
  }

  const upload_file_uri = computed(() => {
    var ns = route.query.ns as string
    return `/api/compose/${ns}/compose/uploadavatar/upload`
  })

  function handleFileUploaded(e) {
    console.log('FileUploaded', e)
    // doFileItemClick({ path: current_path.value.path, isfolder: true });
    let data = e.data
    if (data && data.length > 0) {
      editfrm.value.avatar = data[0].access_url
    }
  }

  function handleDocumentUploaded(e) {
    let data = e.data
    if (data && data.length > 0) {
      var doc = docfrm.value
      doc.download_url = data[0].access_url
      doc.fullpath = data[0].dest_path
      
      var source = data[0].source as string
      var filetype = data[0].file_type as string
      if (source.endsWith("." + filetype)) {
        doc.name = source.substring(0, source.length - filetype.length - 1)
      } else {
        doc.name = source
      }
    }
  }

  function handleVariableManage() {
    showVariableDialog.value = true
  }

  function handleVariableUpdate(t) {
    showVariableDialog.value = t
  }

  function onDelKeys() {
    var ns = route.query.ns as string
    var key = cmdfrm.value.key_prefix ? cmdfrm.value.key_prefix : ''
    onDelKnowledge(key)
  }

  function handleSizeChange(e) {
    PageSize.value = e
    fetch_chunksets()
  }

  function handleDocsSizeChange(e) {
    DocsPageSize.value = e
    fetch_docsets()
  }

  function handleCurrentChange(e) {
    PageCurrent.value = e
    fetch_chunksets()
  }

  function handleDocsCurrentChange(e) {
    DocsPageCurrent.value = e
    fetch_docsets()
  }

  function handleStructCurrentChange(e) {
    StructPageCurrent.value = e
    fetch_structsets()
  }

  function handleStructSizeChange(e) {
    StructPageSize.value = e
    fetch_structsets()
  }

  function onAddChunkDirect() {
    if (current_knowledge.value && current_knowledge.value.id) {
      chunkfrm.value = {
        knowledge_id: current_knowledge.value.id,
      }
      showChunkEditForm.value = true
    } else {
      ElMessage.info("请先选择一个知识库！")
    }
  }

  function onAddDocument() {
    if (current_knowledge.value && current_knowledge.value.id) {
      docfrm.value = {
        knowledge_id: current_knowledge.value.id,
        process_config: {}
      }
      showDocumentEditForm.value = true
    } else {
      ElMessage.info("请先选择一个知识库！")
    }
  }

  function onAddStructDocument() {
    if (current_knowledge.value && current_knowledge.value.id) {
      structfrm.value = {
        knowledge_id: current_knowledge.value.id,
      }
      showStructEditForm.value = true
    } else {
      ElMessage.info("请先选择一个知识库！")
    }
  }

  function onAddKnowledge() {
    editfrm.value = {}
    showEditForm.value = true
  }

  function onEditKnowledge(kb) {
    editfrm.value = kb
    showEditForm.value = true    
  }

  function onEditChunk(row) {
    chunkfrm.value = row
    showChunkEditForm.value = true
  }

  function onEditStruct(row) {
    var ns = route.query.ns as string
    var data = row
    structfrm.value = data
    showStructEditForm.value = true
    showLoading.value = true
    structset_get(ns, row.id).then(res => {
      showLoading.value = false
      if (res.status === 0 || res.status === 200) {
        structfrm.value = res.data
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function onEditDocset(row) {
    var data = row
    try {
      data.process_config = JSON.parse(row.processor)

      if (!data.process_config) {
        data.process_config = {}
      }
    } catch {
      data.process_config = {}
    }
    docfrm.value = data
    console.log('docset', data)
    showDocumentEditForm.value = true
  }

  function onDialogClosed() {
    showEditForm.value = false
    showChunkEditForm.value = false
    showDocumentEditForm.value = false
    showStructEditForm.value = false
  }

  function onConfirm() {
    var es = hook.value
    var ns = route.query.ns as string
    var data = editfrm.value
    showLoading.value = true
    knowledge_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_knowledges()
        showEditForm.value = false
      } else {
        ElMessage.warning("执行失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("执行失败：" + ex.message)
    })
  }

  function onConfirmChunk() {
    var ns = route.query.ns as string
    var data = chunkfrm.value
    data.chunk_status = "PENDING"; // pending to index
    showLoading.value = true
    chunkset_save(ns, data).then(res => {
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
    data.processor = JSON.stringify(data.process_config)
    showLoading.value = true
    docset_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_docsets()
        showDocumentEditForm.value = false
      } else {
        ElMessage.warning("保存失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("保存失败：" + ex.message)
    })
  }

  function onConfirmStruct() {
    var ns = route.query.ns as string
    var data = structfrm.value
    data.status = 1; // pending to index
    showLoading.value = true
    structset_save(ns, data).then(res => {
      showLoading.value = false
      if (res.status === 200 || res.status === 0) {
        fetch_structsets()
        showStructEditForm.value = false
      } else {
        ElMessage.warning("保存失败：" + res.message)
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
      ElMessage.warning("保存失败：" + ex.message)
    })
  }
  
  function onDelStruct(row) {
    var ns = route.query.ns as string
    structset_delete(ns, { id: row.id }).then(res => {
      fetch_chunksets()
    }).catch(ex => {
      console.log(ex)
    })
  }  

  function onDelChunk(row) {
    var ns = route.query.ns as string
    chunkset_delete(ns, {id: row.id}).then(res => {
      fetch_chunksets()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onDelKnowledge(row) {
    var ns = route.query.ns as string
    knowledge_delete(ns, { id: row.id }).then(res => {
      fetch_knowledges()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onDelDocset(key) {
    var ns = route.query.ns as string
    docset_delete(ns, { id: key }).then(res => {
      fetch_docsets()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function onSelectKnowledge(row) {
    var ns = route.query.ns as string
    current_knowledge.value = row
    fetch_chunksets()
    fetch_structsets()
    fetch_docsets()
  }

  function fetch_knowledges() {
    var ns = route.query.ns as string
    var cond = {}
    showLoading.value = true
    knowledge_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        knowledges.value = res.data
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function fetch_chunksets() {
    var ns = route.query.ns as string
    var cond = {
      and: [{field: 'knowledge_id', op: '=', value: current_knowledge.value.id }],
      paging: {
        current: PageCurrent.value,
        size: PageSize.value
      }
    }

    showSimilarity.value = false

    showLoading.value = true
    chunkset_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        chunksets.value = res.data.records
        PageTotal.value = res.data.total
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function fetch_structsets() {
    var ns = route.query.ns as string
    var cond = {
      and: [{field: 'knowledge_id', op: '=', value: current_knowledge.value.id }],
      paging: {
        current: StructPageCurrent.value,
        size: StructPageSize.value
      }
    }

    showSimilarity.value = false

    showLoading.value = true
    structset_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        structsets.value = res.data.records
        StructPageTotal.value = res.data.total
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function fetch_docsets() {
    var ns = route.query.ns as string
    var cond = {
      and: [{field: 'knowledge_id', op: '=', value: current_knowledge.value.id }],
      paging: {
        current: DocsPageCurrent.value,
        size: DocsPageSize.value
      }
    }

    showLoading.value = true
    docset_query(ns, cond).then(res => {
      showLoading.value = false
      if(res.data) {
        documentsets.value = res.data.records
        DocsPageTotal.value = res.data.total
      }
    }).catch(ex => {
      showLoading.value = false
      console.log(ex)
    })
  }

  function onRetrievalDocument() {
    showSimilarity.value = true
    activateDocsTabs.value = "retrieval"
    fetch_retrieval_docs()
  }

  function fetch_retrieval_docs() {
    var ns = route.query.ns as string
    var qv = queryfrm.value
    var data = {
      query: qv.question,
      retrieval_setting: {
        top_k: qv.top_k,
        score_threshold: qv.score_threshold
      }
    }

    showLoading.value = true
    chunkset_retrieval_test(ns, data).then((res: any) => {
      showLoading.value = false
      if(res.records) {
        retrievals.value = res.records
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
    fetch_knowledges()
    onResize()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  