# hx-lsp

[![English](https://img.shields.io/badge/lang-english-blue.svg)](./README.md)

一个提供了自定义代码片段 snippets 和 Code Action 的 lsp 工具。

## 功能

- Completion: 代码片段(snippets)   (helix#9801)
- CodeAction: actions 根据条件执行脚本
- Document Color (helix#12308) 支持文本色彩
- Word Convert case (action)  单词风格转换
	- 下划线(case_snake)
	- 大驼峰(CasePascal)
	- 小驼峰(caseCamel)
- Markdown 支持
	- 表格格式化(Table Formater) (action) ，
			条件为选择区域的第二行为 ` |:-` 组成
	- 粗体(Bold)/斜体(Italic)/删除(Strikethrough )(action)
	- 有序列表(Order),无序列表(Unorder),任务列表(Task List) (action)

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

> 在 <https://github.com/erasin/dotfiles/tree/main/helix/> 中有示例代码，~~另外我自己使用的分支已经合并了 [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801)。~~

## 使用

helix 的语言配置文件 `languages.toml`， 修改下面文件任何一个即可

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

# 或者仅支持部分功能 
language-servers = [ "marksman", "markdown-oxide", { name = "hx-lsp", only-features = [ "document-colors" ] } ]
```


> 关于 `language id` 建议参考 [helix/languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) 文件和 [helix wiki language server configurations](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)。
>
> helix 支持 lsp 使用 `only-features` 和 `except-features ` 来过滤功能。
> hx-lsp 支持
>   - completion
>   - code-action
>   - document-colors


## 配置文件

配置文件支持 `jsonc` 格式，即支持注释内容，但不支持多余的 `,`。

> 注释样式支持 `// ...`, `/* ... */`, `# ...` 。 

**Snippets** 文件加载路径

- `$XDG_CONFIG_HOME/helix/snippets/`
- `WORKSPACE_ROOT/.helix/snippets/`

**Actions** 配置加载路径

- `$XDG_CONFIG_HOME/helix/actions/`
- `WORKSPACE_ROOT/.helix/actions/`

hx-lsp 会在打开文件的时候自动加载配置语言的相关文件。

> 暂不支持配置文件的动态加载，修改配置文件后，可以使用 `:lsp-restart` 重启来重新加载文件。

## Completion: snippets

hx-lsp 的代码片段兼容 [vscode snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) 格式。

同样文件后缀支持全局后缀`{global_name}.code-snippets` 和语言包后缀`{language_id}.json`。

~~为了更好的使用 snippet 建议 heliix 合并 [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801) 以支持 smart-tab。~~

```svgbob
.
└── snippets
    ├── global.code-snippets
    ├── html.json
    └── markdown.json
```

代码片段`snippet`格式：

- **name**: `String`: 唯一内容，用于索引。
- **prefix**: `String` 或 `Vec<String>`: 提供给 helix 编辑器的补全列表使用。
- **body**: `String` 或 `Vec<String>` : 代码片段。
- **description**: `Option<String|Vec<String>>`: 提示内容

```jsonc
{
  "mdbookNote": {
    "prefix": "mdbookNote",
    "body": [
      "```admonish note ${1:title=\"$2\"}",
      "${3:content}",
      "```"
    ],
    "description": "mdbook admonish note"
  },

  "mdbookBob": {
    "prefix": "mdbookBob",
    "body": "```svgbob \n$1\n```",
    "description": "mdbook svgbob "
  },
  "dir": {
    "prefix": "dir",
    "body": [
      "TM_FILENAME: $TM_FILENAME",
      "TM_FILENAME_BASE: $TM_FILENAME_BASE",
      "TM_DIRECTORY: $TM_DIRECTORY",
      "TM_FILEPATH: ${TM_FILEPATH}",
      "RELATIVE_FILEPATH: $RELATIVE_FILEPATH",
      "WORKSPACE_NAME: $WORKSPACE_NAME ",
      "WORKSPACE_FOLDER: $WORKSPACE_FOLDER "
    ],
    "description": "path of current"
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

**Action** 格式：

- **title**: `String`: 显示条目内容
- **filter**: `String` 或 `Vec<String>`: Shell 脚本, 参数是选择区域内容以及替换字段`Variables`，当为空或者返回 `true`，`1`的时候则使用该交互Action。
- **shell**: `String` 或 `Vec<String>`: Shell 脚本，参数是选择区域内容以及替换字段`Variables`，返回字符串则替换选择区域的内容。
- **description**: `Option<String|Vec<String>>`: 提示内容

> 选择区域的内容使用 `Stdio::piped` 传输，在脚本中使用`$(cat)` 捕捉，或者使用替换字段 `$TM_SELECTED_TEXT`。

```jsonc
/* actions/markdown.json */
{
	"bold": {
		"title": "bold",
		"filter": "",
		"shell": ["echo -n \"**${TM_SELECTED_TEXT}**\""],
		"description": "bold"
	},
	"italic": {
		"title": "italic",
		"filter": "",
		"shell": ["echo -n \"_${TM_SELECTED_TEXT}_\""],
		"description": "italic"
	}
}
```

```jsonc
/* actions/go.json */
{
	"run main": {
		"title": "run main",
		"filter": "[[ \"$TM_CURRENT_LINE\" == *main* ]] && echo true || echo false",
		"shell": [
			"alacritty --hold --working-directory ${TM_DIRECTORY} -e go run ${TM_FILENAME};"
			"notify-send \"Golang\" \"RUN: ${TM_FILENAME}\""
		],
		"description": "go run main"
	},
	"run main in tmux": {
		"title": "tmux: go run main",
		"filter": "[[ \"$(cat)\" == *main* ]] && echo true || echo false",
		"shell": [
			"tmux split-window -h -c ${WORKSPACE_FOLDER}; tmux send 'go run ${TM_FILENAME}' Enter"
		],
		"description": "go run main"
	}
}
```

## Variables 字段

> 阅读 [vscode Variables](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_variables)

为 `snippet.body`, `action.filter`, `action.shell` 提供变量字段。

> 支持 `$UUID` 和 `${UUID}` 变量写法。

**path**

- `TM_SELECTED_TEXT` 选择区域的内容
- `TM_CURRENT_LINE` 光标所在行内容
- `TM_CURRENT_WORD` 光标所在单词内容
- `TM_LINE_INDEX` 光标所在行基于0索引
- `TM_LINE_NUMBER` 光标所在行数字
- `TM_FILENAME` 文件名称
- `TM_FILENAME_BASE` 文件名称，无扩展名称
- `TM_DIRECTORY` 当前文档目录
- `TM_FILEPATH` 当前文档的完整路径
- `RELATIVE_FILEPATH` 文档的相对路径
- `CLIPBOARD` 粘贴板内容
- `WORKSPACE_NAME` 工作区或文件夹名称
- `WORKSPACE_FOLDER` 工作区或文件夹路径
- `CURSOR_INDEX` 基于0索引的游标号
- `CURSOR_NUMBER` 基于一个索引的游标号

**时间日期**

- `CURRENT_YEAR` 当前年份
- `CURRENT_YEAR_SHORT`当前年份后两位
- `CURRENT_MONTH` 当权月份，例如 `02`
- `CURRENT_MONTH_NAME` 月份名称 例如 `July`
- `CURRENT_MONTH_NAME_SHORT` 月份名称 例如 `Jul`
- `CURRENT_DATE` 月份中的日期 `01`
- `CURRENT_DAY_NAME` 星期名称 `Monday`
- `CURRENT_DAY_NAME_SHORT` 日期名称 `Mon`
- `CURRENT_HOUR` 24小时 `01`
- `CURRENT_MINUTE` 分钟 `01`
- `CURRENT_SECOND` 秒 `01`
- `CURRENT_SECONDS_UNIX` Unix开始的秒数
- `CURRENT_TIMEZONE_OFFSET` UTC时区偏移

**随机插入值**

- `RANDOM` 
- `RANDOM_HEX`
- `UUID`

## DocumentColor 色彩支持 

- hex
	- #ffffff
- rgb
	- rgb(255, 255, 255) 支持整数
	- rgb(2.0, 255.0, 255.0) 支持浮点值
  - rgb(100%, 0%, 50%) 支持百分比
	- rgba(1.0, 0.0, 0.0, 0.5)
- hsl
	- hsl(240, 50%, 50%) 色相 0-360 度, 饱和度和亮度百分比。
  - hsl(180, 0.5, 0.5) 浮点值
	- hsla(300, 100%, 100%, 0.5) 
- hsv
	- hsv(300, 100%, 100%) 色相 0-360 度, 饱和度和明度百分比。
  - hsv(180, 0.5, 0.5) 浮点值
	- hsva(180, 0.5, 0.5, 0.5) 

## bevy color

- srgb(1.0,0.0,0.0)
- srgba(1.0, 0.0, 0.0, 0.8)
