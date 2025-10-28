<template>
    <el-dialog
      v-model="props.visible"
      title="添加/编辑AppId/AppSecret"
      width="800"
      align-center
      @close="onDialogClosed"
    >
      <el-form :model="hook" label-width="100px" :inline="true" style="max-width: 600px">
        <el-form-item label="App Id">
            <el-input v-model="hook.app_id" style="width: 600px">
              <template #append>
                <el-button icon="Refresh" @click="generate_appid(12)"/>
              </template>
            </el-input>
        </el-form-item>
        <el-form-item label="App Secret">
            <el-input v-model="hook.app_secret" style="width: 600px">
              <template #append>
                <el-button icon="Refresh" @click="generate_appsecret(36)"/>
              </template>
            </el-input>
        </el-form-item>
        <el-form-item label="用户名">
            <el-input v-model="hook.username" style="width: 160px" />
            <el-form-item label="用户Id">
              <el-input v-model="hook.userid" style="width: 160px" />
            </el-form-item>
        </el-form-item>
        <el-form-item label="所属组织">
            <el-input v-model="hook.orgname" style="width: 600px" />
        </el-form-item>
        <el-form-item label="加密验证">
            <el-switch v-model="hook.encryption"/>
        </el-form-item>
        <el-form-item label="Token">
            <el-input type="textarea" v-model="hook.token" :rows="3" style="width: 600px" placeholder="点击Generate Token生成长期有效的JwtToken"/>
            <el-text>可通过请求头Authorization(Bearer xxx)或URL Query (_token=xxx)传递</el-text>
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="onGenerateToken">生成Token</el-button>
          <el-button @click="$emit('update:visible', false)">取消</el-button>
          <el-button type="primary" @click="onConfirm">
            确认
          </el-button>
        </div>
      </template>
    </el-dialog>
  </template>
  
  <script lang="ts" setup name="config">
  import { lang_list, update } from "@/http/modules/management";
  import { md5_hash, number_to_string, random_string, rsa_encrypt } from "@/utils/encryption";
  import { useRoute } from "vue-router";
  import { call_api } from "@/http/modules/common";
  import { mergeProps, onMounted, ref, watch } from "vue";
import { ElMessageBox } from "element-plus";
  const props = defineProps<{ visible: boolean, hook: any }>();
  const emit = defineEmits(['update:visible', 'update:hook', 'datasync'])
  const tables = ref<Array<any>>([])
  const selections = ref<Array<any>>([])
  const query = ref<any>({})
  const route = useRoute()
  const ScriptLangs = ref<Array<any>>([])

  function handleLangList() {  
    lang_list().then(res => {
      ScriptLangs.value = res.data
    }).catch(ex => {
      console.log(ex)
    })
  }
  
  function onDialogClosed() {
    emit('update:visible', false)
  }
  
  function handleSelectionChange(e: any) {
    selections.value = e
  }
  
  function onConfirm() {
    emit('update:visible', false)
    emit('update:hook', props.hook)
  }



  async function onGenerateToken() {
    emit('update:hook', props.hook)
    var secret = props.hook.app_secret
    secret = (new Date()).getTime()
   

    let gewt = {
      app_id: props.hook.app_id,
      app_secret: secret
    }

    let token = number_to_string(secret) + random_string(36);
    props.hook.token = "growth-" + token;
  }

  function generate_appid(len: number) {
    var tx = 'sid' + random_string(len)
    props.hook.app_id = tx
  }

  function generate_appsecret(len: number) {
    var tx = random_string(len)
    props.hook.app_secret = tx
  }  
 
  onMounted(() => {
    handleLangList()
  });
  </script>
  