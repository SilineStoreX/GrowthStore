import { ElMessage, ElMessageBox } from "element-plus";
import { GlobalStore } from "@/stores";
import { useRouter, useRoute, Router } from "vue-router";
/**
 * @description: 校验网络请求状态码
 * @param {Number} status
 * @return void
 */
export const checkStatus = (status: number): void => {
  switch (status) {
    case 400:
      ElMessage.error("请求失败！请您稍后重试");
      break;
    case 401:
      ElMessage.error("登录失效！请您重新登录");
      break;
    case 403:
      ElMessage.error("当前账号无权限访问！");
      break;
    case 404:
      ElMessage.error("你所访问的资源不存在！");
      break;
    case 405:
      ElMessage.error("请求方式错误！请您稍后重试");
      break;
    case 408:
      ElMessage.error("请求超时！请您稍后重试");
      break;
    case 500:
      ElMessage.error("服务异常！");
      break;
    case 502:
      ElMessage.error("网关错误！");
      break;
    case 503:
      ElMessage.error("服务不可用！");
      break;
    case 504:
      ElMessage.error("网关超时！");
      break;
    default:
      ElMessage.error("请求失败！");
  }
};



export const checkStatusForAPI = (status: number, router: Router): void => {
  console.log('Router: ', router)
  switch (status) {
    case 400:
      ElMessage.error("请求失败！请您稍后重试");
      break;
    case 401:
      ElMessageBox.confirm(
        '登录已失效！请前往‘登录与认证配置-->登录与认证测试’重新登录。',
        '警告',
        {
          confirmButtonText: '确定前往',
          cancelButtonText: '关闭',
          type: 'warning',
        }
      ).then(() => {
          router.push("/storex/login?id=global:login")
      })
      break;
    case 403:
      ElMessageBox.confirm(
        '当前账号无权限访问！请前往‘登录与认证配置-->登录与认证测试’重新登录。',
        '警告',
        {
          confirmButtonText: '确定前往',
          cancelButtonText: '关闭',
          type: 'warning',
        }
      ).then(() => {
          router.push("/storex/login?id=global:login")
      })
      break;
    case 404:
      ElMessage.error("你所访问的资源不存在！");
      break;
    case 405:
      ElMessage.error("请求方式错误！请您稍后重试");
      break;
    case 408:
      ElMessage.error("请求超时！请您稍后重试");
      break;
    case 500:
      ElMessage.error("服务异常！");
      break;
    case 502:
      ElMessage.error("网关错误！");
      break;
    case 503:
      ElMessage.error("服务不可用！");
      break;
    case 504:
      ElMessage.error("网关超时！");
      break;
    default:
      ElMessage.error("请求失败！");
  }
};
