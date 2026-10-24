# NPZSee

**快速浏览多维 NumPy 数组的原生桌面工具。**

[English](README.md) · [PyPI](https://pypi.org/project/npzsee/) · [下载](https://github.com/terenceyjh/npzsee/releases) · [VS Code 插件版](https://github.com/terenceyjh/NPZSee-vscode) · [跨平台说明](docs/cross-platform.md)

NPZSee 使用 Rust + egui 编写，可以直接打开 `.npy`、`.npz` 文件。独立程序无需
Python、NumPy、Qt 或 Matplotlib；也提供安装同一个原生程序的 pip 安装包。

本仓库提供可独立运行的 **桌面版**，支持 Windows、macOS 和 Linux。如果你希望直接
在 VS Code 中查看 `.npy`、`.npz` 文件，可以使用配套的
**[NPZSee for VS Code 插件版](https://github.com/terenceyjh/NPZSee-vscode)**。

## 安装与运行

### 通过 pip 安装

NPZSee 已发布到 **[PyPI](https://pypi.org/project/npzsee/)**。使用 Python 3.10 或更新版本，
在终端中安装，然后运行 `npzsee` 即可启动桌面界面：

```sh
python -m pip install npzsee
npzsee
```

也可以直接打开指定文件：

```sh
npzsee "文件路径.npz"
```

pip 会自动选择对应平台的预编译安装包，安装的原生程序与独立下载版相同，无需
NumPy、Qt 或 Matplotlib。没有匹配的预编译包时，从源码构建需要 Rust 和桌面开发库；
平台要求见[跨平台说明](docs/cross-platform.md)。

### 下载独立程序

从 [GitHub Releases](https://github.com/terenceyjh/npzsee/releases) 下载对应系统和处理器的文件，
完整解压后运行：

- Windows：双击 `npzsee.exe`。
- macOS 13 及以上：打开 `NPZSee.app`；M 系列芯片选 arm64，Intel 芯片选 x86_64。
- Linux：在图形桌面终端运行 `./npzsee`，需要 glibc 2.28+ 和可用的 OpenGL 驱动。

打开窗口后选择文件，或将文件拖入窗口；也可以使用 `npzsee "文件路径.npz"`。
当前候选程序没有商业代码签名或 Apple 公证。

## 主要功能

- 左侧列出全部数组，点击后读取；加载、解压和统计在后台执行。
- 多维数组可以逐层进入，二维显示矩阵，一维显示向量，顶部路径可随时返回。
- 默认显示表格；长单元格和 JSON 值点击后在 Table 左侧的详情栏显示、复制完整内容。
- 大表格采用虚拟行和行列分页，最后一行、最后一列均能访问。
- 曲线支持缩放、平移、复数分量、X/Y、对数轴、原始采样值和 SVG 导出。
- 直方图有面积填色，提供明确标注的近似预览，也可选择全量精确统计。
- 支持简体中文、英语、法语、西班牙语、俄语、阿拉伯语，界面顶部可切换并记住选择。
- 文件只读，不执行对象数组中的 pickle。

内存映射、按需解压和按屏幕宽度生成的峰谷曲线，能减少大数组浏览中的等待和绘制量。
性能收益取决于数据、硬件和操作；Rust 本身并不保证 ZIP 解压比 NumPy 更快。

完整格式范围、构建方法与限制见 [英文说明](README.md)。发布步骤见
[发布指南](docs/publishing.md)，当前验证结果见 [发布状态](docs/release-status.md)。

## 来源与许可证

基于并受启发于 [LiukDiihMieu/npyquick](https://github.com/LiukDiihMieu/npyquick)，
感谢原项目提供的 NumPy 数据浏览思路。NPZSee 使用 Rust 重写，并增加多维导航、
后台分析、多语言和原生打包，以独立名称发布，保留原项目版权与来源声明。

使用 GNU GPL v3 或更新版本开源。详见 [LICENSE](LICENSE)、[NOTICE](NOTICE)
以及 [第三方声明](LICENSES/)。发布时提供包含锁定依赖源代码的对应源码归档。
