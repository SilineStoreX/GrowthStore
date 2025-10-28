<template>
    <div class="home">
      <el-tabs v-model="activateAITabs" style="width: 100%" tab-position="left">
          <el-tab-pane label="提示词模板" name="prompts">
            <PromptPanel />
          </el-tab-pane>
          <el-tab-pane label="知识库管理" name="knowledgebase">
            <KnowledgePanel />
          </el-tab-pane>
          <el-tab-pane label="意图模型" name="intents">
            <IntentPanel />
          </el-tab-pane>
          <el-tab-pane label="历史会话" name="memories">
            <MemoryPanel />
          </el-tab-pane>
          <el-tab-pane label="MCP工具" name="toolcalls">
            <ToolcallPanel />
          </el-tab-pane>
          <el-tab-pane label="Chat WebUI" name="growthai-webui">
            <ChatPanel />
          </el-tab-pane>          
      </el-tabs>
    </div>
  </template>
  
  <script lang="ts" setup name="filesystem">
  import { onMounted, ref } from "vue";
  import '@/utils/datetime.js'
  import PromptPanel from "./prompts.vue"
  import MemoryPanel from "./memories.vue"
  import KnowledgePanel from "./knowledges.vue"
  import ToolcallPanel from "./toolcalls.vue"
  import IntentPanel from "./intents.vue"
  import ChatPanel from "./chat.vue"

  const activateAITabs = ref('prompts')

  onMounted(() => {
    
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  