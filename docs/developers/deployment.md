# 本地构建与 Pages 部署

```powershell
pnpm docs:build
pnpm docs:test
pnpm docs:preview
```

默认地址 http://127.0.0.1:4174/HGhub/，产物 dist-docs/，基础路径 /HGhub/。自定义域名可设 HGHUB_DOCS_BASE=/ 后构建。

.github/workflows/docs.yml 仅手动触发，安装锁定依赖、构建、检查链接与包内容，上传静态产物。没有 push 触发器。

在线文档地址为 https://minireel.github.io/HGhub/。仓库 Pages 设置使用 GitHub Actions，推送源码后手动运行 Developer documentation；该工作流只发布静态文档，不构建应用或创建 Release。

若本次源码推送不需要运行应用 CI，可在提交信息中使用 `[skip ci]`，随后单独手动运行文档工作流。不要为文档发布创建版本标签。

站点只复制显式列出的公开文档与示例，不打包 local-plugins、test-results、handoff 和 Android 生成项目。技能包由相同协议源文件生成，避免手工副本漂移。
