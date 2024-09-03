<template>
    <add-app-secret :visible="showVariableDialog" :hook="currentVariable" @update:visible="handleVariableDialogVisibleChange" @update:hook="handleUpdateVariable" />
    <div class="home">
      <div>登录与认证配置</div>
      <div class="tips">登录与认证配置的配置的修改影响面比较广，对这些配置修改后，请重新启动Store X，以确保正确生效。</div>
      <el-form label-position="top" label-width="auto" >
          <el-form-item label="应用名称">
              <el-input v-model="authconf.app_name" />
          </el-form-item>
          <el-form-item label="启用登录/认证服务">
              <el-switch v-model="authconf.enable" />
          </el-form-item>
          <el-form-item label="登录时启用Captcha图像验证">
              <el-switch v-model="authconf.enable_captcha" />
          </el-form-item>
          <el-form-item label="只验证Token，意味着该实例不提供用户登录/以及刷新Token的能力">
              <el-switch v-model="authconf.validate_token_only" />
          </el-form-item>
          <el-form-item label="SSO Token验证URL，如果提供基于SSO的Token验证URL，在验证的过程中会将原始传入的Token转由该接口进行验证">
              <el-input v-model="authconf.validate_url" />
          </el-form-item>
          <el-form-item label="只验证基本信息，大多数情况下只验证基本信息就够用">
              <el-switch v-model="authconf.validate_basic" />
          </el-form-item>
          <el-divider content-position="left">接口与查询配置</el-divider>
          <el-form-item label="用户查询服务，采用StoreX的URI协议定义的方式调用用户查询服务，如object://com.xxxx.user/User">
              <el-input v-model="authconf.user_search" />
          </el-form-item>
          <el-form-item label="角色查询服务，采用StoreX的URI协议定义的方式调用用户查询服务，如query://com.xxxx.user/findRoles">
              <el-input v-model="authconf.role_search" />
          </el-form-item>
          <el-divider content-position="left">用户密码处理</el-divider>
          <el-form-item label="密码加密算法，指对原始密码进行传输和保存过程中的处理办法，其中Mix是对密码进行AES加密，然后再进行MD5处理，加密的密码被存放在数据库中，验证时再加上Captcha进行MD5">
              <el-radio-group v-model="authconf.credential_hash_method">
                <el-radio-button label="MD5" value="md5" />
                <el-radio-button label="SHA1" value="sha1" />
                <el-radio-button label="SHA2" value="sha2" />
                <el-radio-button label="AES" value="aes" />
                <el-radio-button label="Mix" value="mix" />
                <el-radio-button label="RSA" value="rsa" />
              </el-radio-group>
          </el-form-item>
          <el-form-item v-if="authconf.credential_hash_method === 'aes' || authconf.credential_hash_method === 'mix'" label="Credential Solt，在处理密码时使用的盐，或密钥（对AES算法来说）">
            <el-row :gutter="10">
              <el-col :span="12">            
                <el-input v-model="authconf.credential_key" placeholder="密钥"/>
              </el-col>
              <el-col :span="12">
                <el-input v-model="authconf.credential_solt" placeholder="盐"/>
              </el-col>
            </el-row>  
          </el-form-item>
          <el-form-item v-if="authconf.credential_hash_method === 'rsa' || authconf.credential_hash_method === 'dsa'" label="Credential Public Key，RSA或DSA的Public Key">
              <el-input v-model="authconf.credential_key" type="textarea" :rows="5" laceholder="Public Key密钥"/>
          </el-form-item>
          <el-form-item v-if="authconf.credential_hash_method === 'rsa' || authconf.credential_hash_method === 'dsa'" label="Credential Private Key，RSA或DSA的Private Key">
              <el-input v-model="authconf.credential_solt" type="textarea" :rows="5" laceholder="Private Key密钥"/>
          </el-form-item>
          <el-divider content-position="left">JWT 认证配置</el-divider>
          <el-form-item label="JWT Token加密盐">
              <el-input v-model="authconf.token_solt" />
          </el-form-item>
          <el-form-item label="JWT Token过期时长">
              <el-input v-model="authconf.token_expire" />
          </el-form-item>
          <el-form-item label="JWT验证失败也允许请求通过">
              <el-switch v-model="authconf.fail_bypass" />
          </el-form-item>
          <el-divider content-position="left">用户信息字段对应（对应用户查询服务）</el-divider>
          <el-form-item label="user_id字段">
              <el-input v-model="authconf.userid_field" />
          </el-form-item>          
          <el-form-item label="username字段">
              <el-input v-model="authconf.username_field" />
          </el-form-item>
          <el-form-item label="credentials字段（用户密码）">
              <el-input v-model="authconf.user_credentials_field" />
          </el-form-item>          
          <el-form-item label="user_state字段（用户状态，可选）">
              <el-input v-model="authconf.user_state_field" />
          </el-form-item>
          <el-form-item label="user_lock字段（用户是否锁定，可选）">
              <el-input v-model="authconf.user_lock_field" />
          </el-form-item>
          <el-form-item label="reset_pwd_time字段（重置密码时间，可选）">
              <el-input v-model="authconf.reset_pwd_field" />
          </el-form-item>          
          <el-divider content-position="left">角色字段（对应角色查询服务）</el-divider>
          <el-form-item label="role_name字段">
              <el-input v-model="authconf.role_name_field" />
          </el-form-item>
          <el-form-item label="标准ROLE_NAME名称（多个role_name名称使用半角分号[;]分隔）">
              <el-input v-model="authconf.role_name_presets" />
          </el-form-item>
          <el-divider content-position="left">API Secret帐号</el-divider>
          <el-form-item label="启用API Secret帐号体系，可以使用预先提供的AppId和AppSecret来交换Token">
              <el-switch v-model="authconf.enable_api_secure" />
          </el-form-item>
          <el-form-item label="检查关联用户（检查关联用户是否存在）">
              <el-switch v-model="authconf.check_relative_user" />
          </el-form-item>
          <el-form-item label="AppId和AppSecret的提供者（用于查询AppId/AppSecret的InvokeURI，可选。如果没有提供，则会使用下表中自定义AppId/AppSecret）">
              <el-input v-model="authconf.appsecret_provider" />
          </el-form-item>
          <el-form-item v-if="!authconf.appsecret_provider || authconf.appsecret_provider === ''" label="AppId和AppSecret管理">
              <el-table :data="authconf.app_secret_keys" :border="true">
                <el-table-column prop="app_id" label="AppId" />
                <el-table-column prop="app_secret" label="Secret" />
                <el-table-column prop="username" label="关联用户名" />
                <el-table-column prop="orgname" label="所属组织" />
                <el-table-column label="操作" width="120px">
                    <template #default="scoped">
                        <el-button type="primary" icon="Edit" circle @click="handleModifyVariable(scoped.row)" />
                        <el-popconfirm title="确认要删除吗?" @confirm="handleRemoveVariable(scoped.row)">
                            <template #reference>
                                <el-button type="danger" icon="Delete" circle />
                            </template>
                        </el-popconfirm>
                    </template>
                </el-table-column>
              </el-table>
              <el-button @click="onAddVariable">添加</el-button>
          </el-form-item>
          <el-divider content-position="left">SaaS 帐号系统</el-divider>
          <el-form-item label="启用SaaS帐号系统">
              <el-switch v-model="authconf.enable_organization" />
          </el-form-item>
          <el-form-item v-if="authconf.enable_organization" label="组织机构字段">
              <el-input v-model="authconf.organization_field" />
          </el-form-item>
          <el-divider content-position="left">数据权限（行级数据权限）</el-divider>
          <div class="remark">
            <span>数据权限是指对当前登录用户对业务数据行级的访问控制。启用数据权限控制意味着，Store X将会对业务数据的查询注入对应的SQL子句。</span>
            <span>在Store X中的数据权限模型，需要一张权限表（或查询）来进行处理。这个权限表（或查询）承载着对权限控制原语的准确翻译。使用该表与业务表进行关联（INNER JOIN）后，根据当前用户进行查询，即可获得该用户可以访问的业务数据。如下表所示：</span>
            <code key="md">
              <table>
                <tr>
                  <th>id</th>
                  <th>username</th>
                  <th>dept_id</th>
                  <th>unit_id</th>
                  <th>user_id</th>
                </tr>
                <tr>
                  <th>序号</th>
                  <th>登录名，该字段为当前用户</th>
                  <th>可以管理的部门</th>
                  <th>可以管理的单位</th>
                  <th>可以管理的用户</th>
                </tr>
              </table> 
            </code>
            <span>通过该表，即可建立当前登录用户对业务数据的多种控制方式。</span>
            <span>如：业务表tbl_order，里面有一个字段user_id表示创建该订单的用户的ID，我们即可用该权限表与tbl_order表就user_id建立关联，使用当前登录用户的username作为条件来查询出该用户可以管理tbl_order表中的数据。</span>
            <span>在实际查询tbl_order时，就会生成包含了数据权限控制的SQL：</span>
            <p class="sql">select * from tbl_order o on inner join tbl_data_permit __p on o.user_id = __p.user_id where __p.user_id = ?</p>
            <span>同样的，也可以使用dept_id，unit_id这些字段与业务表中的对应字段建立关联关系。</span>
            <span>在实际的应用场景中，我们需要定义最小管理单位的数据，如上表中的最小单位是user_id。</span>
            <span>此处配置好了数据权限后，针对业务表或查询还需要进一步配置。缺省来说，业务表的CRUD以及自定义查询是不自动开启数据权限控制的。</span>
            <p class="line"></p>
          </div>
          <el-form-item label="启用数据权限">
              <el-switch v-model="authconf.data_permission" />
          </el-form-item>
          <el-form-item label="数据权限关联表">
              <el-input v-model="authconf.relative_table" />
          </el-form-item>
          <el-form-item label="用于查询当前用户权限的字段（关联值为当前登录用户的ID）">
              <el-input v-model="authconf.permit_userfield" />
          </el-form-item>
          <el-form-item label="与业务数据关联的字段">
              <el-input v-model="authconf.permit_relative_field" />
          </el-form-item>
      </el-form>
      <div class="drawer-footer">
        <el-button @click="getAuthorization">重置</el-button>
        <el-button type="primary" @click="saveAuthorization">
          保存
        </el-button>
      </div>      
    </div>
  </template>  
  <script lang="ts" setup name="authorization">
  import { authorization_get, authorization_post } from "@/http/modules/management";
  import { ElNotification } from "element-plus";
  import { onMounted, ref } from "vue";
  import AddAppSecret from "./add_appsecret.vue"
  const authconf = ref<any>({})
  const showVariableDialog = ref<boolean>(false)  
  const currentHook = ref<any>()
  const currentVariable = ref<any>()

  const getAuthorization = () => {
    authorization_get().then((res: any) => {
      authconf.value = res.data 
    }).catch((ex: any) => {
      console.log(ex)
    })
  }

  const saveAuthorization = () => {
    authorization_post(authconf.value).then((res: any) => {
      ElNotification({
        title: 'Info',
        message: '登录/认证服务配置保存成功。',
        type: "success",
        duration: 3000,
      })
    }).catch((ex: any) => {
      console.log(ex)
      ElNotification({
        title: 'Info',
        message: '登录/认证服务配置保存失败。' + ex,
        type: "error",
        duration: 3000,
      })
    })
  }

  function handleVariableDialogVisibleChange(e: any) {
    showVariableDialog.value = e
  }  

  function onAddVariable() {
    showVariableDialog.value = true
    currentVariable.value = {}
  }

  function handleRemoveVariable(hk) {
    let cps = authconf.value
    let vars = cps.app_secret_keys
    if (!cps.app_secret_keys) {
        vars = []
    }
    let index = vars.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    vars.splice(index, 1)
    cps.app_secret_keys = vars
    authconf.value = cps
  }

  function handleModifyVariable(hk) {
    currentVariable.value = hk
    showVariableDialog.value = true
  }

  function handleUpdateVariable(hk) {
    let cps = authconf.value
    let vars = cps.app_secret_keys
    if (!cps.app_secret_keys) {
        vars = []
    }

    let index = vars.indexOf(hk) // 找到要删除的元素的索引，此处为 2
    if (index >= 0) {
      vars.splice(index, 1)
    }
    
    vars.push(hk)
    cps.app_secret_keys = vars
    authconf.value = cps
  }

  onMounted(() => {
    console.log("config");
    getAuthorization()
  });
  </script>
  
  <style lang="scss" scoped>
  @import "index.scss";
  </style>
  