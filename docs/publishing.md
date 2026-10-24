# 发布 NPZSee

独立仓库：<https://github.com/terenceyjh/npzsee>。产品显示名为 **NPZSee**，
PyPI 包、命令和 Rust crate 均为 **npzsee**。

## 已准备的发布流程

`.github/workflows/wheels.yml` 在六种对应系统/架构的 runner 上构建、测试、
用 NumPy 校验读取结果，并实际安装 wheel 检查命令。两个 Linux runner 还会
运行六种语言的图形检查。通过后汇总成两组文件：

- `npzsee-pypi`：六个 wheel 和一个源码包，供 PyPI 上传。
- `npzsee-release`：六个平台的独立程序、包含锁定依赖的对应源码包及 SHA256SUMS。

独立程序无需 Python。pip 包安装同一个原生程序，要求 Python 3.10+。
公开代码和下载中均不包含私人数据、工具链、SDK、账号配置或 token。

## 需要你亲自完成的账号步骤

1. 在 <https://pypi.org/account/register/> 注册个人账号，验证邮箱，开启 2FA，保存恢复码。
2. 打开 <https://pypi.org/manage/account/publishing/>，添加新的 **pending publisher**，选择 GitHub：

   | 字段 | 填写内容 |
   | --- | --- |
   | PyPI Project Name | `npzsee` |
   | Owner | `terenceyjh` |
   | Repository name | `npzsee` |
   | Workflow name | `publish-pypi.yml` |
   | Environment name | `pypi` |

   这个名字没有查到已发布同名包，但最终是否可用以 PyPI 的实际验证为准。
   pending publisher 不是名称占位，第一次成功上传才会创建项目。

3. 建议先在 <https://test.pypi.org/account/register/> 注册独立测试账号，并在
   <https://test.pypi.org/manage/account/publishing/> 填写同样内容，只将 Environment name
   改为 `testpypi`。TestPyPI 账号和配置不与正式 PyPI 共用。
4. 在 GitHub 的 **Actions → Publish to PyPI or TestPyPI → Run workflow** 中，填入
   成功的主分支构建运行 ID，首次选择 `testpypi`。所选构建必须是本仓库主分支的
   `wheels.yml` 成功运行；发布工作流会自动校验来源。
5. 从测试站安装检查，确认后用同一个构建 ID 选择 `pypi` 正式发布。

没有必要把账号密码或长期 API Token 写入项目。Trusted Publishing 使用短期身份凭据。
官方说明：[账号要求](https://docs.pypi.org/your-account/managing-your-account/)、
[首次发布配置](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)。

## 安装检查

建议在干净虚拟环境中测试：

```sh
python3 -m venv npzsee-test
npzsee-test/bin/python -m pip install --index-url https://test.pypi.org/simple/ --no-deps npzsee==0.1.0
npzsee-test/bin/npzsee --version
npzsee-test/bin/npzsee
```

Windows 将 `bin/python` 换为 `Scripts/python.exe`，`bin/npzsee` 换为 `Scripts/npzsee.exe`。
正式发布后使用 `python -m pip install npzsee`。

## 本地构建与手动上传（可选）

```sh
python -m pip install maturin twine build
maturin build --release --locked --out dist
python -m build --sdist
python -m twine check dist/*
python scripts/check-artifacts.py
```

每个平台的 wheel 需要对应的系统/架构构建环境；不要将二进制文件改名冒充其他架构。
`python scripts/package-standalone.py dist/<wheel文件名>` 从 wheel 生成独立程序归档。

不用 GitHub 发布时，可以用 PyPI API Token 配合 `python -m twine upload`，首次需能创建
项目的账号级 token，之后可收紧到项目范围。账号注册与 2FA 仍然必须由账号持有人完成。

同名、同版本、同文件名上传后不能覆盖修改；后续更新应提高版本。当前还没有向 PyPI 上传。

## GitHub Release 自动上传

公开测试版下载：<https://github.com/terenceyjh/npzsee/releases/tag/0.1.0>。
六个平台的成功构建 ID 为 **40091597920**，后续配置 PyPI 后直接发布这一组文件。

`.github/workflows/release-github.yml` 可从成功的主分支构建取回已验证产物，核对来源和
校验值，再直接上传 GitHub Release。这样无需从本机反复上传大文件。此次发布流程：
<https://github.com/terenceyjh/npzsee/actions/runs/40093347966>。

后续版本在 GitHub Actions 中选择 **Publish GitHub release**，填写成功的构建 ID 和
与包版本一致的 tag。发布说明放在 `docs/releases/<版本>.md`。流程只接受成功的
主分支构建，且不会覆盖已经公开的同名 Release。
