# GrowthStore

GrowthStore是一个基于Rust体系的成长型的企业级后台服务的开发框架。它提供了“约定大于配置，配置即服务”的开发理念，采用统一的约定、灵活的配置来实现绝大多数的业务，通过灵活高效的扩展体系，来满足各种特殊化的业务需求，快速应对企业的业务成长需要。GrowthStore的出现，为企业级后端开发注入了新鲜的血液。 

## GrowthStore 二进制安装
在github的release库中有预先编译好的二进制安装包，所有的都采用tar.gz格式提供。 
如果，你要下载的二进制包是用于Linux环境的，你需要确认你的运行环境与Release库的Linux二进制包构建环境类似。

### 安装前的准备
- 首先，你需要下载GrowthStore的Windows安装包，Windows下的安装包只有x64版，不提供x86的32位版本。
- 其次，确定你需要进行安装的目录，GrowthStore运行过程中，主要产生较大量磁盘需求的是日志功能；你可以根据具体情况来分配磁盘空间。
- 最后，如果你是更新的话，请记得事先备份好，<安装目录>/assets/models/ 和 <安装目录>/assets/configs/ 下的文件。

### Windows下的安装
- 解压下载的Zip包文件到你的安装目录；
- 进入到你的安装目录；
- 双击执行 store-server，这个时候会开启一个终端窗口
- 如果终端窗口闪退，说明运行过程中出现了异常，很大的原因是监听的端口已被占用造成的，这时，可以进入目录<安装目录>/assets/configs/下，修改Config.toml文件中的listen章节中的port的值。
- 运行成功后，打开浏览器，在地址栏中输入 http://localhost:17800/ ，来验证一下是否安装成功，安装成功的话，这个时候会显示Store X的管理员登录界面；

### Linux 下的安装
- 解压下载的tar.gz包文件到你的安装目录；
- 进入到你的安装目录；
- 运行 store-server；
- 如果闪退，说明运行过程中出现了异常，很大的原因是监听的端口已被占用造成的，这时，可以进入目录<安装目录>/assets/configs/下，修改Config.toml文件中的listen章节中的port的值。
- 运行成功后，打开浏览器，在地址栏中输入 http://localhost:17800/ ，来验证一下是否安装成功，安装成功的话，这个时候会显示Store X的管理员登录界面；
- 在<安装目录>下运行，需要如下方式执行：
```bash
./store-server
./store-server & #在后台执行
nohup ./store-server &  #重定向输入输出，且在后台执行，一般来说，相让其在后台运行，我们应该使用该方法
```

# GrowthStore 源代码安装
如果你选择从源代码进行安装，则需要阅读本章节。

### 开发环境的准备
为了能够编译GrowthStore，需要进行一下开发环境的准备。
#### Windows 下的准备
在Windows下，主要是需要准备一套可以用于链接的C/C++运行环境。通常，我们会选择下载微软的Visual Studio 2022 (至少需要2019版)。可以从下面的地址下载Visual Studio，且，在安装时选择Visual C/C++的开发环境；

#### Linux 下的准备
在Linux下，主要是需要准备一套可以用于链接的C/C++运行环境。通常，我们可以安装gcc/g++。
``` bash
# ubuntu
sudo apt install gcc
# redhat
sudo yum install gcc
```

#### Rust 下载与安装
可以进入Rust官网，按照官网( https://www.rust-lang.org/learn/get-started )指示来进行安装。
安装完成后，可以在终端执行如下命令来验证：
``` bash
cargo -v
rustc -v
```

#### NodeJS 下载与安装
可以进入NodeJS官网，按照官网( https://nodejs.org/en/download/package-manager )指示来进行安装。
这里，我们需要选择 18.20.4 及以上版本。
然后，安装npm，yarn等工具。

#### GrowthStore的编译
从github仓库或gitee仓库中clone出一份最新的代码。然后，进入到clone后的项目目录，执行如下：
``` bash
# 如果要构建debug版，目标生成在target/debug/目录下
cargo b

# 如果要构建release版，目标生成在target/release/目录下
cargo b -r
```
初次编译，会需要很长的时间。请耐心等待，同时，在执行下载的过程中，会由于网络传输等原因中断构建，这个时候，你只需要重复执行上述命令即可。

如果你长时间没有进行对Store X进行构建了，这个时候需要对Store X的依赖包进行更新，再来执行构建，可以执行如下命令。
``` bash
cargo update
cargo b

```

#### GrowthStore前端的构建
进行项目目录的front子目录，执行：
``` bash
yarn install 
yarn run build:pro
```
打包成功后，生成的前端文件放在 <项目目录>/front/dist下。

#### GrowthStore安装
```
<安装目录>
├─ assets
│  ├─ configs
│  ├─ certs
│  ├─ metadata
|  ├─ models
|  ├─ scripts
|  └─ www
├─ logs
└─ store-server
```
其中 assets/configs下主要用于存放Authorization.toml和Config.toml这两个全局配置文件。assets/certs下用于存放SSL证书（如果需要启用TLS的话）。assets/models用于存放运行时的配置项，是非常重要的目录。assets/metadata则是用于存放一些中间生成的元数据的信息，系统自动维护；assets/scripts下主要用于存放一些脚本文件。assets/www 目录用于存放前端文件。logs目录用于存放日志文件。
<br/>完成前端的编译后，即可将<项目目录>/front/dist下的文件，复制到<安装目录>/assets/www/下；
<br/>完成Rust构建后，即可将<项目目录>/target/release/store-server，复制到<安装目录>/下；
<br/>接下，在<安装目录>/下运行store-server，运行成功后，打开浏览器，在地址栏中输入 http://localhost:17800/ ，来验证一下是否安装成功，安装成功的话，这个时候会显示Store X的管理员登录界面；


