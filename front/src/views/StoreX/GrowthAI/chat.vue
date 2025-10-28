<template>
    <div class="home">
        <el-form v-model="queryfrm" :inline="true">
          <el-form-item label="按下空格键识别语音">
            <el-switch v-model="queryfrm.space_recording" size="default" @change="onCreateGrowthAI"></el-switch>
          </el-form-item>          
          <el-form-item label="使用Streaming接口">
            <el-switch v-model="queryfrm.streaming" size="default" @change="onCreateGrowthAI"></el-switch>
          </el-form-item>
          <el-form-item label="浮动对话框">
            <el-switch v-model="queryfrm.floating" size="default" @change="onCreateGrowthAI"></el-switch>
          </el-form-item>
          <el-form-item label="最大化隐藏浮动球">
            <el-switch v-model="queryfrm.hidden_ball_maximum" size="default" @change="onCreateGrowthAI"></el-switch>
          </el-form-item>
          <el-form-item label="以Base64的方式传递文件">
            <el-switch v-model="queryfrm.pass_file_asbase64" size="default" @change="onCreateGrowthAI"></el-switch>
          </el-form-item>
        </el-form>
        <div id="growthai_chat_window"></div>
        <el-image-viewer
          v-if="showPreviewDialog"
          :url-list="previewImageList"
          :initial-index="previewImageIndex"
          show-progress
          @close="showPreviewDialog = false"
        />
    </div>
  </template>
  
  <script lang="ts" setup name="filesystem">
  import { onMounted, ref } from "vue";
  import '@/utils/datetime.js'
  import GrowthAI from "growthai-webui"
  import { useRoute } from "vue-router";
  import { fetch_pluginnames, config_get } from "@/http/modules/management";
  import "growthai-webui/dist/css/style.css"
  import { GlobalStore } from "@/stores";
  import { v4 as uuidv4} from "uuid"


  const route = useRoute()
  const default_rag_plugin = ref("rag")
  const rest_config = ref<any>({})
  const showPreviewDialog = ref<boolean>(false)
  const globalStore = GlobalStore();
  const previewImageList = ref<Array<any>>([])
  const previewImageIndex = ref<any>(0)
  const queryfrm = ref({space_recording: false, streaming: true, floating: false, hidden_ball_maximum: true, pass_file_asbase64: false})

  function get_auth_token() {
    return globalStore.api_token
  }

  function onCreateGrowthAI() {
    createGrowthAI()
  }


  function createGrowthAI() {
    var ns = route.query.ns as string
    let chat_uri = rest_config.value.chat_base_uri + 'chat/sse'
    let voiceasr_url = rest_config.value.voice_base_uri + 'recognize/upload'

    if (!queryfrm.value.streaming) {
      chat_uri = rest_config.value.chat_base_uri + 'chat'
    }

    if (globalStore.chat_conversation_id) {
      let uuid_ = uuidv4()
      globalStore.setChatConversationId(uuid_)
    }
    let chat_id = globalStore.chat_conversation_id;

    var growth = document.getElementById("growthai-1")
    if (growth) {
      growth.parentNode.removeChild(growth)
    }

    var chat_container = document.getElementById('growthai_chat_window')
    if (chat_container) {
      chat_container.innerHTML = ''
    }

    
    var upload_endpoint = `/api/compose/${ns}/compose/uploadavatar/upload`
    
    new GrowthAI({
      ainame: 'GrowthAI',
      growth_id: 'growthai-1',
      use_websocket: false,
      authorization: get_auth_token(),
      frame_placement: '#growthai_chat_window',
      adjust_frame_height: 220,
      allow_images: true,
      allow_documents: true,
      pass_file_asbase64: queryfrm.value.pass_file_asbase64,
      space_recording: queryfrm.value.space_recording,
      hidden_ball_maximum: queryfrm.value.hidden_ball_maximum,
      attach_on_create: !queryfrm.value.floating,
      streaming: queryfrm.value.streaming,
      upload_endpoint: upload_endpoint,
      voiceasr_endpoint: voiceasr_url,
      handleSendMessage: (msg: string, attachs: Array<any>) => {
        var chatmsgs: any[] = [{"role":"user","content": { "Text": msg }, "metadata": { "custom": {}}}]
        var meadies: any[] = []
         if (attachs) {
            for (var att of attachs) {
                if (queryfrm.value.pass_file_asbase64) {
                    var base64image = {
                      "Image": {
                        type: "image_url",
                        format: "base64",
                        image_url: att.base64data + ''
                      }
                    }
                    meadies.push(base64image)
                } else {
                    if (att.image_url) {
                        var image = {
                          "Image": {
                            type: "image_url",
                            image_url: att.image_url  + ''
                          }
                        }
                        meadies.push(image)
                    } else if (att.base64data) {
                        var base64image = {
                          "Image": {
                            type: "image_url",
                            format: "base64",
                            image_url: att.base64data  + ''
                          }
                        }
                        meadies.push(base64image)
                    }
                }
            }
        }

        if (meadies.length > 0) {
          chatmsgs.push({"role": "user", "content": { "MultiModal": meadies }, "metadata": { "custom": {} }})
        }

        return {
                "messages": chatmsgs,
                "conversation_id": chat_id,
                "stream": queryfrm.value.streaming
              }
      },
      handleReceivedMessage: (msg: any) => {
        if (queryfrm.value.streaming) {
          if (msg.choices && msg.choices.length > 0) {
            return {
              content: msg.choices[0].message?.content?.delta?.content,
              thinking: msg.choices[0].message?.content?.delta?.reasoning_content 
            }
          } else {
            if (msg.error_code >= 400) {
              return msg.error_msg
            }
          }
        } else {
          if (msg.error_code === 0 || msg.error_code === 200) {
            return msg.records?.content?.Text
          } else {
            return msg.error_msg
          }
        }
        return ""
      },
      previewImageHandle: (image_src: string, image_list: string[]) => {
        console.log('preview images: ', image_src, image_list)
        previewImageList.value = image_list
        var i = image_list.indexOf(image_src)
        if (i >= 0) {
          previewImageIndex.value = i
        } else {
          previewImageIndex.value = 0
        }
        showPreviewDialog.value = true
      },
      handleRefineFileURL: (file_url: string) => {
        return file_url + '?_token=' + get_auth_token()
      },
      sse_endpoint: chat_uri,
    }).create({})
  }

  function fetchPluginNames() {
    var ns = route.query.ns as string
    fetch_pluginnames(ns, "rag").then(res => {
      let plugins = res.data
      if (plugins.length >= 1) {
        default_rag_plugin.value = plugins[0]
      }

      get_rag_config()
    }).catch(ex => {
      console.log(ex)
    })
  }

  function get_rag_config() {
    var ns = route.query.ns as string
    var name = default_rag_plugin.value
    config_get("rag", ns, name).then(res => {
      rest_config.value = res.data || {}
      createGrowthAI()
    }).catch(ex => {
      console.log(ex)
    })
  }

  onMounted(() => {
    fetchPluginNames()
  });
  </script>
  
  <style lang="scss" scoped>
  @use "index.scss";
  </style>
  