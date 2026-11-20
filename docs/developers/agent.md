# 让 Agent 协助开发

下载[HGhub 插件开发技能包](../../downloads/hghub-plugin-dev.zip)和[安装器](../../downloads/install.py)，放在同一目录。包只包含公开协议、支持状态、类型和虚构示例。

```powershell
python install.py --archive hghub-plugin-dev.zip --destination "$HOME/.codex/skills"
```

也可将 ZIP 内 hghub-plugin-dev 文件夹复制到 Agent 技能目录。安装器拒绝覆盖同名目录，升级前先备份旧目录。

## 可复制提示词

```text
请从本页下载 HGhub 插件开发技能包，解压到我指定的 Agent 技能目录，阅读 SKILL.md 和 references/status.md。
使用 hghub-plugin-dev 为我指定的站点开发单文件 HPP 插件。先核实实际接口与播放链，再实现存在的能力；使用 hg.http、hg.storage 与资源对象，保留真实作品、播放列表和分集 ID。
交付插件 JS、manifest.json 和可重复验证步骤，分别说明模拟、真实宿主、真实上游与设备播放的结果。
如果当前对话没有目标站点或输出目录，请先问我；已有信息直接使用。不发布或上传，除非我明确要求。
```

代码块提供复制按钮。下载链接相对当前站点解析；本地构建验证不代表尚未发布的公网地址已可访问。
