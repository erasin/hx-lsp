# hx-lsp

[![English](https://img.shields.io/badge/lang-english-blue.svg)](./README.md)


## install 

```sh
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

修改 helix 配置文件 `$XDG_CONFIG_HOME/helix/languages.toml` 或者 项目目录下 `.helix/languages.toml`， 根据对应的语言追加 `language-servers` 配置。

比如 markdown

```toml
[language-server.hx-lsp]
command = "hx-lsp"

[[language]]
name = "markdown"
language-servers = ["hx-lsp"]
```

## snippets 自定义代码片段

snippet 定义为兼容 [vscode snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) 格式。这样就可以直接和 vscode 通用片段。

为了更好的使用 snippet 建议 heliix 合并 [helix#9081](https://github.com/helix-editor/helix/pull/9801) 以支持 smart-tab。

文件加载路径顺序为：

- [ ] `WORKSPACE_ROOT/.helix/snippets/`
- [o] `$XDG_CONFIG_HOME/helix/snippets/`

加载的文件为 `语言id.json`, 和 `xxx.code-snippets` 全局文件。

- 语言文件,比如 markdown.json, javascript.json , go.json
- 全局文件，比如 global.code-snippets


## Actions 自定义



