# Chimes Store X Server

### 介绍 📖

用于学习 vue3+ts

### 项目功能 🔨

- 使用 Vue3.2 + TypeScript 开发
- 使用 Vite3 作为项目开发、打包工具（配置跨域代理……）
- 使用 Pinia 替代 Vuex，集成 Pinia 持久化插件
- 使用 Axios 并二次封装常用请求
- 使用 Element-Plus 全局注册组件、修改主题
- 使用 vue-i18n 国际化
- 使用 three.js 满足 3d 需求
- 使用 VueRouter 进行路由权限拦截、路由懒加载，包含分两种布局方式
- 使用 keepAlive 对页面进行缓存
- 使用 vscode 插件 Prettier 统一格式化代码

### 使用 📔

- **Clone：**

```text
# Gitee
git clone https://gitee.com/zhen_xin_ting/vue3-demo
```

- **Install：**

```text
npm install
```

- **Run：**

```text
npm run dev
npm run serve
```

- **Build：**

```text
# 开发环境
npm run build:dev

# 生产环境
npm run build:pro
```

### 文件资源目录 📚

```text
chimes-store
├─ .vscode                # VSCode 推荐配置
├─ public                 # 静态资源文件（该文件夹不会被打包）
├─ src
│  ├─ assets              # 静态资源文件
│  ├─ components          # 全局组件
│  ├─ config              # 全局配置项
│  ├─ enums               # 项目常用枚举
│  ├─ http                # API 接口管理
│  ├─ i18n                # 语言国际化 i18n
│  ├─ json                # json文件（假数据）
│  ├─ layouts             # 框架布局模块
│  ├─ routers             # 路由管理
│  ├─ stores              # pinia store
│  ├─ style               # 全局scss样式表
│  ├─ typings             # 全局 ts 声明
│  ├─ utils               # 常用工具库
│  ├─ views               # 项目所有页面
│  ├─ App.vue             # 项目主组件
│  ├─ main.ts             # 项目入口文件
│  └─ vite-env.d.ts       # 指定 ts 识别 vue
├─ .env.development       # 开发环境配置
├─ .env.production        # 生产环境配置
├─ .gitignore             # 忽略 git 提交
├─ index.html             # 入口 html
├─ package-lock.json      # 依赖包包版本锁
├─ README.md              # README 介绍
├─ tsconfig.json          # typescript 全局配置
├─ tsconfig.node.json     # typescript 编译选项配置说明
└─ vite.config.ts         # vite 全局配置文件
```

### 项目后台接口 🧩

项目接口采用 Mock 数据

- EasyMock：https://mock.mengxuegu.com

### 这是一个Vue3的WebSite
- https://gitee.com/todays-mai-chen-han/portal-v3
