import { userInfo } from "os";

// 请求响应参数(不包含data)
export interface Result {
  status: any;
  message: string;
  timestamp: any;
}

// 请求响应参数(包含data)
export interface ResultData<T = any> extends Result {
  data: T;
}

// 登录模块
export namespace Login {
  export interface ReqLoginForm {
    username: string;
    password: string;
  }
  export interface ResLogin {
    status: number;
    token: string;
    msg: string;
    userInfo: userInfo;
  }
}

// 用户模块

export interface userInfo {
  id: string;
  username: string;
  fullname: string;
  avatar: string;
}
