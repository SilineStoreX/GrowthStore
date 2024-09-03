<template>
    <license :visible="showLicenseDialog" title="个人信息保护条款" src="/SERVICE-PERSONAL-PRIVACY" :hook="currentHook" @update:visible="handleDialogVisibleChange"></license>    
    <el-dialog
      v-model="props.visible"
      :title="props.title"
      width="800"
      align-center
      :close-on-click-modal="false"
      :close-on-press-escape="false"
      class="reg-dialog"
      style="padding-left: 60px; padding-right: 60px; padding-top: 30px; padding-bottom: 40px;"
      @close="onDialogClosed"
    >
      <el-form ref="contactFormRef" :model="hook" :rules="rules" :inline="false" label-position="top">
            <span class="tips">注意：注册联系人信息不是注册登录用户，其目的是为了方便我们联系您，并推送Store X的最新更新。</span>
            <el-form-item label="您的姓名" prop="fullname">
                <el-input v-model="hook.fullname" placeholder="请输入您的姓名"/>
            </el-form-item>
            <el-form-item label="您的手机号" prop="contact_phone">
                <el-input v-model="hook.contact_phone" placeholder="请输入可以联系到您的手机号码或电话号码"/>
            </el-form-item>
            <el-form-item label="您的邮箱" prop="contact_email">
                <el-input v-model="hook.contact_email" placeholder="请输入可以联系到您的邮箱"/>
            </el-form-item>
            <el-form-item label="单位名称" prop="company_name">
                <el-input v-model="hook.company_name" placeholder="请输入您所在单位名称" />
            </el-form-item>
            <el-form-item label="所属行业" prop="industry">
                <el-cascader v-model="hook.industry" :options="industryOptions" placeholder="请选择所属行业" style="width: 100%;"/>
            </el-form-item>
            <el-form-item label="单位规模" prop="company_size">
                <el-select v-model="hook.company_size" placeholder="请选择公司规模">
                    <el-option v-for="it in companySize" :value="it">{{ it }}</el-option>
                </el-select>
            </el-form-item>
            <el-form-item label="您的职位" prop="position">
                <el-select v-model="hook.position" placeholder="请选择您的职位">
                    <el-option v-for="it in positions" :value="it">{{ it }}</el-option>
                </el-select>
            </el-form-item>                                                                    
            <el-form-item label="所在地区" prop="area">
                <el-cascader v-model="hook.area" :options="areaOptions" style="width: 100%;" placeholder="请选择所在地区"/>
            </el-form-item>
            <el-form-item>
                <span><el-checkbox v-model="hook.acceptance" /> 我已阅读并接受<a href="#" @click="onShowLicenseDialog">《个人信息保护条款》</a>，同意相关个人信息传输。StoreX有权在法律允许的范围内对活动进行解释</span>
            </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="info" @click="onCancel(contactFormRef)">取消</el-button>
          <el-button type="primary" @click="onConfirmSave(contactFormRef)">确定</el-button>
        </div>
      </template>
    </el-dialog>
  </template>
  
  <script lang="ts" setup name="config">
  import { onMounted, reactive, ref } from "vue";
  import { call_api_options } from "@/http/modules/common";
  import { ElMessage, ElMessageBox, ElNotification, FormInstance, FormRules } from "element-plus";
  import License from "./license.vue"

  const props = defineProps<{ visible: boolean, hook: any, title: string }>();
  const emit = defineEmits(['update:visible', 'update:hook', 'datasync'])
  const selections = ref<Array<any>>([])
  const rules = reactive<FormRules>({
    fullname: [{ required: true, message: "请输入您的姓名" }],
    contact_phone: [{ required: true, message: "请输入您的手机号" }],
    contact_email: [{ required: true, message: "请输入您的邮箱" }],
    company_name: [{ required: true, message: "请输入单位名称" }],
    industry: [{ required: true, message: "请输入所属行业" }],
    company_size: [{ required: true, message: "请输入单位规模" }],
    position: [{ required: true, message: "请输入您的职位" }],
    area: [{ required: true, message: "请输入所在地区" }],
  });
  const contactFormRef = ref();
  const currentHook = ref({})
  const showLicenseDialog = ref(false);
  const companySize = ref<Array<any>>([
    "10人以下", "10到50人", "50到100人", "100到200人", "200到500人", "500到1000人", "1000人以上"
  ])

  const positions = ref<Array<any>>([
    "首席执行官/总裁", "总经理", "首席信息官", "首席技术官", "首席数据官", "首席运营官", "首席营销官", "首席财务官", "首席数据官",
    "高级副总裁/副总裁", "部门负责人", "总监/高级总监", "经理/高级经理", "工程师", "教师/讲师", "教授/副教授", "学生", "员工", "其他"
  ])

  const industryOptions = ref<Array<any>>([
    {
        value: '房地产',
        label: '房地产',
        children: [
        {
            value: '综合地产集团',
            label: '综合地产集团',
        },
        {
            value: '商业地产',
            label: '商业地产',
        },
        {
            value: '物业管理',
            label: '物业管理',
        },
        {
            value: '园区',
            label: '园区',
        },
        {
            value: '房地产服务',
            label: '房地产服务',
        },
        ]
    },
    {
        value: '建筑工程',
        label: '建筑工程',
    },
    {
        value: '金融',
        label: '金融',
        children: [
        {
            value: '银行',
            label: '银行',
        },
        {
            value: '保险',
            label: '保险',
        },
        {
            value: 'PEVC',
            label: 'PEVC',
        },
        {
            value: '证券',
            label: '证券',
        },
        {
            value: '基金',
            label: '基金',
        },
        {
            value: '期货',
            label: '期货',
        },
        {
            value: '金融服务',
            label: '金融服务',
        },
        {
            value: '消费金融',
            label: '消费金融',
        },
        ]
    },
    {
        value: '快销品',
        label: '快销品',
        children: [
        {
            value: '家庭清洁',
            label: '家庭清洁',
        },
        {
            value: '美妆个护',
            label: '美妆个护',
        },
        {
            value: '母婴用品',
            label: '母婴用品',
        },
        {
            value: '食品饮料酒',
            label: '食品饮料酒',
        },
        {
            value: '烟草',
            label: '烟草',
        },
        ]
    },
    {
        value: '服务与纺织',
        label: '服务与纺织'
    },
    {
        value: '家居生活方式',
        label: '家居生活方式'
    },
    {
        value: '文娱传媒',
        label: '文娱传媒',
        children: [
        {
            value: '新闻电视媒体',
            label: '新闻电视媒体',
        },
        {
            value: '广告营销',
            label: '广告营销',
        },
        {
            value: '游戏内容社交',
            label: '游戏内容社交',
        },
        {
            value: '影视综艺娱乐',
            label: '影视综艺娱乐',
        },
        ]
    },
    {
        value: '软件/硬件/IT服务',
        label: '软件/硬件/IT服务',
        children: [
        {
            value: '通用软件',
            label: '通用软件',
        },
        {
            value: '垂直行业软件',
            label: '垂直行业软件',
        },
        {
            value: '数据中心产业链',
            label: '数据中心产业链',
        },
        {
            value: '集成商',
            label: '集成商',
        },
        {
            value: '外包服务',
            label: '外包服务',
        },
        {
            value: '单位IT部门',
            label: '单位IT部门',
        },
        ]
    },
    {
        value: '电子商务和服务',
        label: '电子商务和服务',
        children: [
        {
            value: '电商平台',
            label: '电商平台',
        },
        {
            value: '线上生活平台',
            label: '线上生活平台',
        },
        {
            value: '在线教育',
            label: '在线教育',
        },
        {
            value: 'B2B平台/服务',
            label: 'B2B平台/服务',
        },
        ]
    },
    {
        value: '线下零售服务',
        label: '线下零售服务',
        children: [
        {
            value: '餐饮',
            label: '餐饮',
        },
        {
            value: '酒店',
            label: '酒店',
        },
        {
            value: '生活服务',
            label: '生活服务',
        },
        {
            value: '百货',
            label: '百货',
        },
        {
            value: '超市/连锁超市',
            label: '超市/连锁超市',
        },
        {
            value: '便利店/连锁便利店',
            label: '便利店/连锁便利店',
        },
        ]
    },
    {
        value: '汽车',
        label: '汽车',
        children: [
        {
            value: '新能源汽车',
            label: '新能源汽车',
        },
        {
            value: '传统整车',
            label: '传统整车',
        },
        {
            value: '电池/电机/电控',
            label: '电池/电机/电控',
        },
        {
            value: '雷达和传感器',
            label: '雷达和传感器',
        },
        {
            value: '智能汽车技术',
            label: '智能汽车技术',
        },
        {
            value: '其他汽车零部件',
            label: '其他汽车零部件',
        },
        {
            value: '汽车服务',
            label: '汽车服务',
        },
        ]
    },
    {
        value: '消费电子及零部件',
        label: '消费电子及零部件',
        children: [
        {
            value: '手机/数码产品',
            label: '手机/数码产品',
        },
        {
            value: '芯片半导体',
            label: '芯片半导体',
        },
        {
            value: '其他零部件',
            label: '其他零部件',
        },
        ]
    },
    {
        value: '家电',
        label: '家电',
    },
    {
        value: '医疗和健康',
        label: '医疗和健康',
        children: [
        {
            value: '中成药/保健品',
            label: '中成药/保健品',
        },
        {
            value: '制药/制品',
            label: '制药/制品',
        },
        {
            value: '生物科技',
            label: '生物科技',
        },
        {
            value: '医药流通与销售',
            label: '医药流通与销售',
        },
        {
            value: '医药机构与服务',
            label: '医药机构与服务',
        },
        {
            value: '医疗器械',
            label: '医疗器械',
        },
        ]
    },
    {
        value: '采矿业',
        label: '采矿业',
        children: [
        {
            value: '金属',
            label: '金属',
        },
        {
            value: '非金属',
            label: '非金属',
        },
        {
            value: '煤碳',
            label: '煤碳',
        },
        {
            value: '石油和天燃气',
            label: '石油和天燃气',
        },
        ]
    },
    {
        value: '制造业',
        label: '制造业',
        children: [
        {
            value: '化工',
            label: '化工',
        },
        {
            value: '电气设备',
            label: '电气设备',
        },
        {
            value: '通用和专业设备',
            label: '通用和专业设备',
        },
        {
            value: '基础材料和其它',
            label: '基础材料和其它',
        },
        {
            value: '交通运输设备',
            label: '交通运输设备',
        },
        {
            value: '机器人和无人机',
            label: '机器人和无人机',
        },
        {
            value: '航空航天',
            label: '航空航天',
        },
        {
            value: '军工',
            label: '军工',
        },
        ]
    },
    {
        value: '能源',
        label: '能源',
        children: [
        {
            value: '传统能源',
            label: '传统能源',
        },
        {
            value: '新能源',
            label: '新能源',
        },
        {
            value: '电池',
            label: '电池',
        },
        {
            value: '氢能源',
            label: '氢能源',
        },
        {
            value: '核能源',
            label: '核能源',
        },
        ]
    },
    {
        value: '农业',
        label: '农业',
    },
    {
        value: '通信',
        label: '通信',
    },
    {
        value: '教育',
        label: '教育',
    },
    {
        value: '专业服务',
        label: '专业服务',
    },
    {
        value: '运输/物流业',
        label: '运输/物流业',
        children: [
        {
            value: '物流',
            label: '物流',
        },
        {
            value: '水陆交通',
            label: '水陆交通',
        },
        {
            value: '航空',
            label: '航空',
        },
        {
            value: '低空服务',
            label: '低空服务',
        },
        ]
    },
    {
        value: '公共事业',
        label: '公共事业',
        children: [
        {
            value: '水和环保',
            label: '水和环保',
        },
        {
            value: '事业单位',
            label: '事业单位',
        },
        {
            value: '非赢利组织',
            label: '非赢利组织',
        },
        {
            value: '其它',
            label: '其它',
        },
        ]
    },
  ])

  const areaOptions = ref<Array<any>>([{"label": "北京市", "value": "110000000000", "children": [{"label": "北京市", "value": "110100000000"}]}, {"label": "天津市", "value": "120000000000", "children": [{"label": "天津市", "value": "120100000000"}]}, {"label": "河北省", "value": "130000000000", "children": [{"label": "石家庄市", "value": "130100000000"}, {"label": "唐山市", "value": "130200000000"}, {"label": "秦皇岛市", "value": "130300000000"}, {"label": "邯郸市", "value": "130400000000"}, {"label": "邢台市", "value": "130500000000"}, {"label": "保定市", "value": "130600000000"}, {"label": "张家口市", "value": "130700000000"}, {"label": "承德市", "value": "130800000000"}, {"label": "沧州市", "value": "130900000000"}, {"label": "廊坊市", "value": "131000000000"}, {"label": "衡水市", "value": "131100000000"}]}, {"label": "山西省", "value": "140000000000", "children": [{"label": "太原市", "value": "140100000000"}, {"label": "大同市", "value": "140200000000"}, {"label": "阳泉市", "value": "140300000000"}, {"label": "长治市", "value": "140400000000"}, {"label": "晋城市", "value": "140500000000"}, {"label": "朔州市", "value": "140600000000"}, {"label": "晋中市", "value": "140700000000"}, {"label": "运城市", "value": "140800000000"}, {"label": "忻州市", "value": "140900000000"}, {"label": "临汾市", "value": "141000000000"}, {"label": "吕梁市", "value": "141100000000"}]}, {"label": "内蒙古自治区", "value": "150000000000", "children": [{"label": "呼和浩特市", "value": "150100000000"}, {"label": "包头市", "value": "150200000000"}, {"label": "乌海市", "value": "150300000000"}, {"label": "赤峰市", "value": "150400000000"}, {"label": "通辽市", "value": "150500000000"}, {"label": "鄂尔多斯市", "value": "150600000000"}, {"label": "呼伦贝尔市", "value": "150700000000"}, {"label": "巴彦淖尔市", "value": "150800000000"}, {"label": "乌兰察布市", "value": "150900000000"}, {"label": "兴安盟", "value": "152200000000"}, {"label": "锡林郭勒盟", "value": "152500000000"}, {"label": "阿拉善盟", "value": "152900000000"}]}, {"label": "辽宁省", "value": "210000000000", "children": [{"label": "沈阳市", "value": "210100000000"}, {"label": "大连市", "value": "210200000000"}, {"label": "鞍山市", "value": "210300000000"}, {"label": "抚顺市", "value": "210400000000"}, {"label": "本溪市", "value": "210500000000"}, {"label": "丹东市", "value": "210600000000"}, {"label": "锦州市", "value": "210700000000"}, {"label": "营口市", "value": "210800000000"}, {"label": "阜新市", "value": "210900000000"}, {"label": "辽阳市", "value": "211000000000"}, {"label": "盘锦市", "value": "211100000000"}, {"label": "铁岭市", "value": "211200000000"}, {"label": "朝阳市", "value": "211300000000"}, {"label": "葫芦岛市", "value": "211400000000"}]}, {"label": "吉林省", "value": "220000000000", "children": [{"label": "长春市", "value": "220100000000"}, {"label": "吉林市", "value": "220200000000"}, {"label": "四平市", "value": "220300000000"}, {"label": "辽源市", "value": "220400000000"}, {"label": "通化市", "value": "220500000000"}, {"label": "白山市", "value": "220600000000"}, {"label": "松原市", "value": "220700000000"}, {"label": "白城市", "value": "220800000000"}, {"label": "延边朝鲜族自治州", "value": "222400000000"}]}, {"label": "黑龙江省", "value": "230000000000", "children": [{"label": "哈尔滨市", "value": "230100000000"}, {"label": "齐齐哈尔市", "value": "230200000000"}, {"label": "鸡西市", "value": "230300000000"}, {"label": "鹤岗市", "value": "230400000000"}, {"label": "双鸭山市", "value": "230500000000"}, {"label": "大庆市", "value": "230600000000"}, {"label": "伊春市", "value": "230700000000"}, {"label": "佳木斯市", "value": "230800000000"}, {"label": "七台河市", "value": "230900000000"}, {"label": "牡丹江市", "value": "231000000000"}, {"label": "黑河市", "value": "231100000000"}, {"label": "绥化市", "value": "231200000000"}, {"label": "大兴安岭地区", "value": "232700000000"}]}, {"label": "上海市", "value": "310000000000", "children": [{"label": "上海市", "value": "310100000000"}]}, {"label": "江苏省", "value": "320000000000", "children": [{"label": "南京市", "value": "320100000000"}, {"label": "无锡市", "value": "320200000000"}, {"label": "徐州市", "value": "320300000000"}, {"label": "常州市", "value": "320400000000"}, {"label": "苏州市", "value": "320500000000"}, {"label": "南通市", "value": "320600000000"}, {"label": "连云港市", "value": "320700000000"}, {"label": "淮安市", "value": "320800000000"}, {"label": "盐城市", "value": "320900000000"}, {"label": "扬州市", "value": "321000000000"}, {"label": "镇江市", "value": "321100000000"}, {"label": "泰州市", "value": "321200000000"}, {"label": "宿迁市", "value": "321300000000"}]}, {"label": "浙江省", "value": "330000000000", "children": [{"label": "杭州市", "value": "330100000000"}, {"label": "宁波市", "value": "330200000000"}, {"label": "温州市", "value": "330300000000"}, {"label": "嘉兴市", "value": "330400000000"}, {"label": "湖州市", "value": "330500000000"}, {"label": "绍兴市", "value": "330600000000"}, {"label": "金华市", "value": "330700000000"}, {"label": "衢州市", "value": "330800000000"}, {"label": "舟山市", "value": "330900000000"}, {"label": "台州市", "value": "331000000000"}, {"label": "丽水市", "value": "331100000000"}]}, {"label": "安徽省", "value": "340000000000", "children": [{"label": "合肥市", "value": "340100000000"}, {"label": "芜湖市", "value": "340200000000"}, {"label": "蚌埠市", "value": "340300000000"}, {"label": "淮南市", "value": "340400000000"}, {"label": "马鞍山市", "value": "340500000000"}, {"label": "淮北市", "value": "340600000000"}, {"label": "铜陵市", "value": "340700000000"}, {"label": "安庆市", "value": "340800000000"}, {"label": "黄山市", "value": "341000000000"}, {"label": "滁州市", "value": "341100000000"}, {"label": "阜阳市", "value": "341200000000"}, {"label": "宿州市", "value": "341300000000"}, {"label": "六安市", "value": "341500000000"}, {"label": "亳州市", "value": "341600000000"}, {"label": "池州市", "value": "341700000000"}, {"label": "宣城市", "value": "341800000000"}]}, {"label": "福建省", "value": "350000000000", "children": [{"label": "福州市", "value": "350100000000"}, {"label": "厦门市", "value": "350200000000"}, {"label": "莆田市", "value": "350300000000"}, {"label": "三明市", "value": "350400000000"}, {"label": "泉州市", "value": "350500000000"}, {"label": "漳州市", "value": "350600000000"}, {"label": "南平市", "value": "350700000000"}, {"label": "龙岩市", "value": "350800000000"}, {"label": "宁德市", "value": "350900000000"}]}, {"label": "江西省", "value": "360000000000", "children": [{"label": "南昌市", "value": "360100000000"}, {"label": "景德镇市", "value": "360200000000"}, {"label": "萍乡市", "value": "360300000000"}, {"label": "九江市", "value": "360400000000"}, {"label": "新余市", "value": "360500000000"}, {"label": "鹰潭市", "value": "360600000000"}, {"label": "赣州市", "value": "360700000000"}, {"label": "吉安市", "value": "360800000000"}, {"label": "宜春市", "value": "360900000000"}, {"label": "抚州市", "value": "361000000000"}, {"label": "上饶市", "value": "361100000000"}]}, {"label": "山东省", "value": "370000000000", "children": [{"label": "济南市", "value": "370100000000"}, {"label": "青岛市", "value": "370200000000"}, {"label": "淄博市", "value": "370300000000"}, {"label": "枣庄市", "value": "370400000000"}, {"label": "东营市", "value": "370500000000"}, {"label": "烟台市", "value": "370600000000"}, {"label": "潍坊市", "value": "370700000000"}, {"label": "济宁市", "value": "370800000000"}, {"label": "泰安市", "value": "370900000000"}, {"label": "威海市", "value": "371000000000"}, {"label": "日照市", "value": "371100000000"}, {"label": "临沂市", "value": "371300000000"}, {"label": "德州市", "value": "371400000000"}, {"label": "聊城市", "value": "371500000000"}, {"label": "滨州市", "value": "371600000000"}, {"label": "菏泽市", "value": "371700000000"}]}, {"label": "河南省", "value": "410000000000", "children": [{"label": "郑州市", "value": "410100000000"}, {"label": "开封市", "value": "410200000000"}, {"label": "洛阳市", "value": "410300000000"}, {"label": "平顶山市", "value": "410400000000"}, {"label": "安阳市", "value": "410500000000"}, {"label": "鹤壁市", "value": "410600000000"}, {"label": "新乡市", "value": "410700000000"}, {"label": "焦作市", "value": "410800000000"}, {"label": "濮阳市", "value": "410900000000"}, {"label": "许昌市", "value": "411000000000"}, {"label": "漯河市", "value": "411100000000"}, {"label": "三门峡市", "value": "411200000000"}, {"label": "南阳市", "value": "411300000000"}, {"label": "商丘市", "value": "411400000000"}, {"label": "信阳市", "value": "411500000000"}, {"label": "周口市", "value": "411600000000"}, {"label": "驻马店市", "value": "411700000000"}, {"label": "省直辖县级行政区划", "value": "419000000000"}]}, {"label": "湖北省", "value": "420000000000", "children": [{"label": "武汉市", "value": "420100000000"}, {"label": "黄石市", "value": "420200000000"}, {"label": "十堰市", "value": "420300000000"}, {"label": "宜昌市", "value": "420500000000"}, {"label": "襄阳市", "value": "420600000000"}, {"label": "鄂州市", "value": "420700000000"}, {"label": "荆门市", "value": "420800000000"}, {"label": "孝感市", "value": "420900000000"}, {"label": "荆州市", "value": "421000000000"}, {"label": "黄冈市", "value": "421100000000"}, {"label": "咸宁市", "value": "421200000000"}, {"label": "随州市", "value": "421300000000"}, {"label": "恩施土家族苗族自治州", "value": "422800000000"}, {"label": "省直辖县级行政区划", "value": "429000000000"}]}, {"label": "湖南省", "value": "430000000000", "children": [{"label": "长沙市", "value": "430100000000"}, {"label": "株洲市", "value": "430200000000"}, {"label": "湘潭市", "value": "430300000000"}, {"label": "衡阳市", "value": "430400000000"}, {"label": "邵阳市", "value": "430500000000"}, {"label": "岳阳市", "value": "430600000000"}, {"label": "常德市", "value": "430700000000"}, {"label": "张家界市", "value": "430800000000"}, {"label": "益阳市", "value": "430900000000"}, {"label": "郴州市", "value": "431000000000"}, {"label": "永州市", "value": "431100000000"}, {"label": "怀化市", "value": "431200000000"}, {"label": "娄底市", "value": "431300000000"}, {"label": "湘西土家族苗族自治州", "value": "433100000000"}]}, {"label": "广东省", "value": "440000000000", "children": [{"label": "广州市", "value": "440100000000"}, {"label": "韶关市", "value": "440200000000"}, {"label": "深圳市", "value": "440300000000"}, {"label": "珠海市", "value": "440400000000"}, {"label": "汕头市", "value": "440500000000"}, {"label": "佛山市", "value": "440600000000"}, {"label": "江门市", "value": "440700000000"}, {"label": "湛江市", "value": "440800000000"}, {"label": "茂名市", "value": "440900000000"}, {"label": "肇庆市", "value": "441200000000"}, {"label": "惠州市", "value": "441300000000"}, {"label": "梅州市", "value": "441400000000"}, {"label": "汕尾市", "value": "441500000000"}, {"label": "河源市", "value": "441600000000"}, {"label": "阳江市", "value": "441700000000"}, {"label": "清远市", "value": "441800000000"}, {"label": "东莞市", "value": "441900000000"}, {"label": "中山市", "value": "442000000000"}, {"label": "潮州市", "value": "445100000000"}, {"label": "揭阳市", "value": "445200000000"}, {"label": "云浮市", "value": "445300000000"}]}, {"label": "广西壮族自治区", "value": "450000000000", "children": [{"label": "南宁市", "value": "450100000000"}, {"label": "柳州市", "value": "450200000000"}, {"label": "桂林市", "value": "450300000000"}, {"label": "梧州市", "value": "450400000000"}, {"label": "北海市", "value": "450500000000"}, {"label": "防城港市", "value": "450600000000"}, {"label": "钦州市", "value": "450700000000"}, {"label": "贵港市", "value": "450800000000"}, {"label": "玉林市", "value": "450900000000"}, {"label": "百色市", "value": "451000000000"}, {"label": "贺州市", "value": "451100000000"}, {"label": "河池市", "value": "451200000000"}, {"label": "来宾市", "value": "451300000000"}, {"label": "崇左市", "value": "451400000000"}]}, {"label": "海南省", "value": "460000000000", "children": [{"label": "海口市", "value": "460100000000"}, {"label": "三亚市", "value": "460200000000"}, {"label": "三沙市", "value": "460300000000"}, {"label": "儋州市", "value": "460400000000"}, {"label": "省直辖县级行政区划", "value": "469000000000"}]}, {"label": "重庆市", "value": "500000000000", "children": [{"label": "重庆市", "value": "500100000000"}, {"label": "县", "value": "500200000000"}]}, {"label": "四川省", "value": "510000000000", "children": [{"label": "成都市", "value": "510100000000"}, {"label": "自贡市", "value": "510300000000"}, {"label": "攀枝花市", "value": "510400000000"}, {"label": "泸州市", "value": "510500000000"}, {"label": "德阳市", "value": "510600000000"}, {"label": "绵阳市", "value": "510700000000"}, {"label": "广元市", "value": "510800000000"}, {"label": "遂宁市", "value": "510900000000"}, {"label": "内江市", "value": "511000000000"}, {"label": "乐山市", "value": "511100000000"}, {"label": "南充市", "value": "511300000000"}, {"label": "眉山市", "value": "511400000000"}, {"label": "宜宾市", "value": "511500000000"}, {"label": "广安市", "value": "511600000000"}, {"label": "达州市", "value": "511700000000"}, {"label": "雅安市", "value": "511800000000"}, {"label": "巴中市", "value": "511900000000"}, {"label": "资阳市", "value": "512000000000"}, {"label": "阿坝藏族羌族自治州", "value": "513200000000"}, {"label": "甘孜藏族自治州", "value": "513300000000"}, {"label": "凉山彝族自治州", "value": "513400000000"}]}, {"label": "贵州省", "value": "520000000000", "children": [{"label": "贵阳市", "value": "520100000000"}, {"label": "六盘水市", "value": "520200000000"}, {"label": "遵义市", "value": "520300000000"}, {"label": "安顺市", "value": "520400000000"}, {"label": "毕节市", "value": "520500000000"}, {"label": "铜仁市", "value": "520600000000"}, {"label": "黔西南布依族苗族自治州", "value": "522300000000"}, {"label": "黔东南苗族侗族自治州", "value": "522600000000"}, {"label": "黔南布依族苗族自治州", "value": "522700000000"}]}, {"label": "云南省", "value": "530000000000", "children": [{"label": "昆明市", "value": "530100000000"}, {"label": "曲靖市", "value": "530300000000"}, {"label": "玉溪市", "value": "530400000000"}, {"label": "保山市", "value": "530500000000"}, {"label": "昭通市", "value": "530600000000"}, {"label": "丽江市", "value": "530700000000"}, {"label": "普洱市", "value": "530800000000"}, {"label": "临沧市", "value": "530900000000"}, {"label": "楚雄彝族自治州", "value": "532300000000"}, {"label": "红河哈尼族彝族自治州", "value": "532500000000"}, {"label": "文山壮族苗族自治州", "value": "532600000000"}, {"label": "西双版纳傣族自治州", "value": "532800000000"}, {"label": "大理白族自治州", "value": "532900000000"}, {"label": "德宏傣族景颇族自治州", "value": "533100000000"}, {"label": "怒江傈僳族自治州", "value": "533300000000"}, {"label": "迪庆藏族自治州", "value": "533400000000"}]}, {"label": "西藏自治区", "value": "540000000000", "children": [{"label": "拉萨市", "value": "540100000000"}, {"label": "日喀则市", "value": "540200000000"}, {"label": "昌都市", "value": "540300000000"}, {"label": "林芝市", "value": "540400000000"}, {"label": "山南市", "value": "540500000000"}, {"label": "那曲市", "value": "540600000000"}, {"label": "阿里地区", "value": "542500000000"}]}, {"label": "陕西省", "value": "610000000000", "children": [{"label": "西安市", "value": "610100000000"}, {"label": "铜川市", "value": "610200000000"}, {"label": "宝鸡市", "value": "610300000000"}, {"label": "咸阳市", "value": "610400000000"}, {"label": "渭南市", "value": "610500000000"}, {"label": "延安市", "value": "610600000000"}, {"label": "汉中市", "value": "610700000000"}, {"label": "榆林市", "value": "610800000000"}, {"label": "安康市", "value": "610900000000"}, {"label": "商洛市", "value": "611000000000"}]}, {"label": "甘肃省", "value": "620000000000", "children": [{"label": "兰州市", "value": "620100000000"}, {"label": "嘉峪关市", "value": "620200000000"}, {"label": "金昌市", "value": "620300000000"}, {"label": "白银市", "value": "620400000000"}, {"label": "天水市", "value": "620500000000"}, {"label": "武威市", "value": "620600000000"}, {"label": "张掖市", "value": "620700000000"}, {"label": "平凉市", "value": "620800000000"}, {"label": "酒泉市", "value": "620900000000"}, {"label": "庆阳市", "value": "621000000000"}, {"label": "定西市", "value": "621100000000"}, {"label": "陇南市", "value": "621200000000"}, {"label": "临夏回族自治州", "value": "622900000000"}, {"label": "甘南藏族自治州", "value": "623000000000"}]}, {"label": "青海省", "value": "630000000000", "children": [{"label": "西宁市", "value": "630100000000"}, {"label": "海东市", "value": "630200000000"}, {"label": "海北藏族自治州", "value": "632200000000"}, {"label": "黄南藏族自治州", "value": "632300000000"}, {"label": "海南藏族自治州", "value": "632500000000"}, {"label": "果洛藏族自治州", "value": "632600000000"}, {"label": "玉树藏族自治州", "value": "632700000000"}, {"label": "海西蒙古族藏族自治州", "value": "632800000000"}]}, {"label": "宁夏回族自治区", "value": "640000000000", "children": [{"label": "银川市", "value": "640100000000"}, {"label": "石嘴山市", "value": "640200000000"}, {"label": "吴忠市", "value": "640300000000"}, {"label": "固原市", "value": "640400000000"}, {"label": "中卫市", "value": "640500000000"}]}, {"label": "新疆维吾尔自治区", "value": "650000000000", "children": [{"label": "乌鲁木齐市", "value": "650100000000"}, {"label": "克拉玛依市", "value": "650200000000"}, {"label": "吐鲁番市", "value": "650400000000"}, {"label": "哈密市", "value": "650500000000"}, {"label": "昌吉回族自治州", "value": "652300000000"}, {"label": "博尔塔拉蒙古自治州", "value": "652700000000"}, {"label": "巴音郭楞蒙古自治州", "value": "652800000000"}, {"label": "阿克苏地区", "value": "652900000000"}, {"label": "克孜勒苏柯尔克孜自治州", "value": "653000000000"}, {"label": "喀什地区", "value": "653100000000"}, {"label": "和田地区", "value": "653200000000"}, {"label": "伊犁哈萨克自治州", "value": "654000000000"}, {"label": "塔城地区", "value": "654200000000"}, {"label": "阿勒泰地区", "value": "654300000000"}, {"label": "自治区直辖县级行政区划", "value": "659000000000"}]}])  

  function onDialogClosed() {
    emit('update:visible', false)
  }

  const REMOTE_CONTACT_SERVER = 'https://storex.enjoylost.com'

  async function exchangeJwtToken() {
    let app = {
        app_id: 'sidKm8D31_2n6ea',
        app_secret: 'Gfy9euKqxKFZ8Jb6jguZLDq5hq4sXTlpChhi',
    }
    let jwt = await call_api_options(REMOTE_CONTACT_SERVER + '/api/auth/exchange', 'GET', app, {});
    if (jwt.status === 0 || jwt.status === 200) {
        return jwt.data.token
    } else {
        ElMessage.error('无效的AppId或AppSecret！') 
    }
  }

  function onCancel(formel: FormInstance | undefined) {
    emit('update:visible', false)
    if(formel) {
        formel.resetFields()
    }
  }

  async function onConfirmSave(formEl: FormInstance | undefined) {
    let contact = { ...props.hook }

    if (!contact.acceptance) {
        ElMessageBox.alert("请同意《个人信息保护条款》")
        return
    }

    formEl.validate(async (valid: boolean, _invalidFields?: any) => {
        if (!valid) {
            console.log('_invalidFields', _invalidFields)
            return;
        }
        let jwtoken = await exchangeJwtToken()
        if (jwtoken) {
            let data = await call_api_options(REMOTE_CONTACT_SERVER + '/api/object/com.siline.storex/ContactRegister/upsert', 'POST', contact, { anotherToken: jwtoken});
            if (data.status === 0 || data.status === 200) {
                ElMessage.success('保存成功！')
                formEl.resetFields()
                onDialogClosed()
            } else {
                ElMessage.error('保存失败！请稍候重试！') 
            }
        }
    });
  }

  const handleDialogVisibleChange = (t: boolean) => {
    showLicenseDialog.value = t
  }  
  
  const onShowLicenseDialog = () => {
    showLicenseDialog.value = true    
  }
  
  onMounted(() => {
  });
  </script>
  <style lang="scss" scoped>
  .reg-dialog {
    padding-left: 40px;
    padding-right: 40px;
    height: calc(100vh - 40px);
  }
  .tips {
    display: block;
    padding: 10px;
  }
  </style>