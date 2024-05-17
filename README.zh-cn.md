# hx-lsp

[![English](https://img.shields.io/badge/lang-english-blue.svg)](./README.md)


一个提供了自定义代码片段 snippets 和 Code Action 的 lsp 工具。

## 功能

- Completion: snippets
- CodeAction: actions

## 安装

**从crate安装**

```sh
cargo install --force hx-lsp
```

**源码编译**

```sh
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

> 在 https://github.com/erasin/helix-config/ 中有示例代码，另外我自己使用的分支已经合并了 [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801)。

## 使用


修改 helix 的语言配置文件 `languages.toml`， 修改下面文件任何一个即可

- `$XDG_CONFIG_HOME/helix/languages.toml` helix 配置文件
- `WORKSPACE_ROOT/.helix/languages.toml` 项目下配置文件

> 关于 `WORKSPACE_ROOT`, 获取 helix 提供的 `initialize` 中 `rootPath`, 所以在存在多个 `.helix` 层级的时候读取的是最近的一个 root .

比如为 markdown 追加支持。

```toml
[language-server.hx-lsp]
command = "hx-lsp"

[[language]]
name = "markdown"
language-servers = [ "marksman", "markdown-oxide", "hx-lsp" ]
```

> 关于 `language id` 建议参考 [helix/languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) 文件和 [helix wiki language server configurations](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)。


## 配置文件

配置文件支持 `jsonc` 格式，即支持注释内容，但不支持多余的 `,`。

> 注释样式支持 `// ...`, `/* ... */`, `# ...` 。 

Snippets 文件加载路径

- `$XDG_CONFIG_HOME/helix/snippets/`
-  `WORKSPACE_ROOT/.helix/snippets/`

Actions 配置加载路径

- `$XDG_CONFIG_HOME/helix/actions/`
- `WORKSPACE_ROOT/.helix/actions/`

配置在 `textDocument/didOpen` 时候加载 `language id` 同名 `lang_id.json` 文件。

> 暂不支持配置文件的动态加载，修改配置文件后，可以使用 `:lsp-restart` 重启来重新加载文件。

## Completion: snippets

Code snippets 
兼容 [vscode snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) 格式。同样文件后缀支持 全局后缀`.code-snippets` 和 语言包后缀`.json`。

为了更好的使用 snippet 建议 heliix 合并 [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801) 以支持 smart-tab。

```svgbob
.
└── snippets
    ├── global.code-snippets
    ├── html.json
    └── markdown.json
```

snippet 格式：

- **name**: `String` 唯一内容，用于索引
- **prefix**: `String` 或 `Vec<String>` 提供给 helix 编辑器的补全列表使用
- **body**: `String` 或 `Vec<String>` 
- **description**: `Option<String>` 提示内容

```jsonc
{
  "markdown a": { // name
    "prefix": "mda", // string
    "body": "mda in .helix: ${1:abc} : ${2:cde}", // string
    "description": "test a info content in .helix"
  },
  "markdown b": {
    "prefix": [ // array
      "mdb" 
    ],
    "body": "mdb: ${1:abc} : ${2:cde}", // string
    "description": "test b info content"
  },
  "markdown c": {
    "prefix": [ // array
      "mdc",
      "mdd"
    ],
    "body": [ // array
      "mda: ${1:abc} : ${2:cde}",
      "test"
    ],
    "description": "test c,d info content"
  }
}
```

## CodeAction: actions

```svgbob
.
└── actions
    ├── html.json
    └── markdown.json
````

snippet 格式：

- **title**: `String` helix 显示条目内容
- **catch**: `String` 捕捉内容，regex 适配内容的时候，显示 code action
- **shell**: `String` 或 `Vec<String>` 执行的 shell 脚本
- **description**: `Option<String>` 提示内容


```jsonc
/* actions/go.json */
{
  "run main": {
    "title": "go run main",
    "catch": "func main",
    "shell": [
      "alacritty --hold --working-directory ${TM_DIRECTORY} -e go run ${TM_FILENAME}"
    ],
    "description": "go run main"
  }
}
```

```json
/* test */
{
  "tmux split window helix": {
    "title": "tmux split window in project",
    "catch": "fn",
    "shell": [
      "tmux split-window -h",
      "tmux send \"cd ${WORKSPACE_FOLDER}\n\""
    ],
    "description": "tmux split and open helix in project"
  }
}
```

**catch**：

- [x] 捕捉行
- [ ] 选择内容
- [ ] 匹配内容


## Variables 字段

计划为 snippet body 和 action shell 支持替换字段处理。

支持 `$UUID` 和 `${UUID}` 写法。

**path**

- [x] `TM_SELECTED_TEXT`
- [x] `TM_CURRENT_LINE`
- [x] `TM_CURRENT_WORD`
- [x] `TM_LINE_INDEX`
- [x] `TM_LINE_NUMBER`
- [x] `TM_FILENAME`
- [x] `TM_FILENAME_BASE`
- [x] `TM_DIRECTORY`
- [x] `TM_FILEPATH`
- [x] `RELATIVE_FILEPATH`
- [x] `CLIPBOARD`
- [x] `WORKSPACE_NAME`
- [x] `WORKSPACE_FOLDER`

**time**

- [x] `CURRENT_YEAR`
- [x] `CURRENT_YEAR_SHORT`
- [x] `CURRENT_MONTH`
- [x] `CURRENT_MONTH_NAME`
- [x] `CURRENT_MONTH_NAME_SHORT`
- [x] `CURRENT_DATE`
- [x] `CURRENT_DAY_NAME`
- [x] `CURRENT_DAY_NAME_SHORT`
- [x] `CURRENT_HOUR`
- [x] `CURRENT_MINUTE`
- [x] `CURRENT_SECOND`
- [x] `CURRENT_SECONDS_UNIX`
- [x] `CURRENT_TIMEZONE_OFFSET`

**other**

- [x] `RANDOM`
- [x] `RANDOM_HEX`
- [x] `UUID`

**action catch**

- [ ] `CATCH1..9`
