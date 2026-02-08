# hx-lsp

[![English](https://img.shields.io/badge/lang-english-blue.svg)](./README.md)

一个为 [Helix Editor](https://github.com/helix-editor/helix) 提供代码片段（snippets）和代码操作（Code Actions）的 LSP 工具。

---

## 功能特性

### LSP 命令

- `reload snippets` - 重新加载代码片段配置
- `reload actions` - 重新加载代码操作配置

### 核心功能

| 功能 | 说明 | 相关 PR |
|------|------|---------|
| **代码补全** | 支持 VSCode 格式的代码片段 | [helix#9801](https://github.com/helix-editor/helix/pull/9801) |
| **代码操作** | 自定义 Shell 脚本操作 | - |
| **文档颜色** | 识别并显示 CSS/Bevy 颜色 | [helix#12308](https://github.com/helix-editor/helix/pull/12308) |
| **单词风格转换** | 下划线、驼峰、大驼峰转换 | - |

### Markdown 专属功能

- **表格格式化** - 自动对齐表格列（选择区域需包含表头分隔符 `|:-`）
- **文本样式** - 粗体、斜体、删除线
- **列表转换** - 有序列表、无序列表、任务列表

---

## 安装

### 从 crates.io 安装（推荐）

```bash
cargo install --force hx-lsp
```

### 从源码编译

```bash
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

---

## 配置

### Helix 语言配置

编辑 Helix 的语言配置文件 `languages.toml`：

- 全局配置：`$XDG_CONFIG_HOME/helix/languages.toml`
- 项目配置：`WORKSPACE_ROOT/.helix/languages.toml`

> **关于 `WORKSPACE_ROOT`**：从 Helix 提供的 `initialize` 请求中的 `rootPath` 获取。当存在多层级的 `.helix` 目录时，会读取最近的一层。

#### 配置示例

为 Markdown 添加 hx-lsp 支持：

```toml
[language-server.hx-lsp]
command = "hx-lsp"

[[language]]
name = "markdown"
language-servers = ["marksman", "markdown-oxide", "hx-lsp"]

# 或者仅启用部分功能
language-servers = [
  "marksman",
  "markdown-oxide",
  { name = "hx-lsp", only-features = ["document-colors"] }
]
```

> **关于 `language id`**：参考 [helix/languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) 和 [Helix Wiki](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)。

Helix 支持使用 `only-features` 和 `except-features` 过滤 LSP 功能，hx-lsp 支持以下功能：
- `completion` - 代码补全
- `code-action` - 代码操作
- `document-colors` - 文档颜色

---

## 配置文件

配置文件使用 `jsonc` 格式（支持注释，但不支持尾随逗号）。

> **支持的注释格式**：`// ...`、`/* ... */`、`# ...`

### 文件加载路径

**代码片段（Snippets）**：
- 全局：`$XDG_CONFIG_HOME/helix/snippets/`
- 项目：`WORKSPACE_ROOT/.helix/snippets/`

**代码操作（Actions）**：
- 全局：`$XDG_CONFIG_HOME/helix/actions/`
- 项目：`WORKSPACE_ROOT/.helix/actions/`

当 LSP 收到 `textDocument/didOpen` 请求时，会自动加载对应语言的配置文件。

> 使用 Helix 命令 `:lsp-workspace-command` 可以唤起命令选择器，手动重载 snippets 或 actions。

---

## 代码片段（Snippets）

hx-lsp 兼容 [VSCode Snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) 格式。

### 文件命名规则

- 全局片段：`{name}.code-snippets`
- 语言专属：`{language_id}.json`

```
snippets/
├── global.code-snippets    # 全局代码片段
├── html.json              # HTML 代码片段
└── markdown.json          # Markdown 代码片段
```

### Snippet 格式

| 字段 | 类型 | 说明 |
|------|------|------|
| `prefix` | `String` 或 `String[]` | 触发补全的关键词 |
| `body` | `String` 或 `String[]` | 代码片段内容 |
| `description` | `String` 或 `String[]` | 描述信息（可选） |

### 示例

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
    "body": "```svgbob\n$1\n```",
    "description": "mdbook svgbob"
  },

  "dir": {
    "prefix": "dir",
    "body": [
      "TM_FILENAME: $TM_FILENAME",
      "TM_FILENAME_BASE: $TM_FILENAME_BASE",
      "TM_DIRECTORY: $TM_DIRECTORY",
      "TM_FILEPATH: ${TM_FILEPATH}",
      "RELATIVE_FILEPATH: $RELATIVE_FILEPATH",
      "WORKSPACE_NAME: $WORKSPACE_NAME",
      "WORKSPACE_FOLDER: $WORKSPACE_FOLDER"
    ],
    "description": "显示当前文件路径信息"
  }
}
```

---

## 代码操作（Actions）

Actions 允许根据条件执行 Shell 脚本，并将输出结果插入到编辑器中。

```
actions/
├── html.json
└── markdown.json
```

### Action 格式

| 字段 | 类型 | 说明 |
|------|------|------|
| `title` | `String` | 在 Helix 中显示的标题 |
| `filter` | `String` 或 `String[]` | Shell 脚本，返回 `true`、`1` 或空字符串时启用该 Action |
| `shell` | `String` 或 `String[]` | Shell 脚本，输出结果将替换选中文本 |
| `description` | `String` 或 `String[]` | 描述信息（可选） |

> **注意**：选中的文本通过 `Stdio::piped` 传递给脚本，可以使用 `$(cat)` 捕获，或使用变量 `$TM_SELECTED_TEXT`。

### 示例

**Markdown 文本格式化**：

```jsonc
/* actions/markdown.json */
{
  "bold": {
    "title": "加粗",
    "filter": "",
    "shell": ["echo -n \"**${TM_SELECTED_TEXT}**\""],
    "description": "将选中文本加粗"
  },
  "italic": {
    "title": "斜体",
    "filter": "",
    "shell": ["echo -n \"_${TM_SELECTED_TEXT}_\""],
    "description": "将选中文本设为斜体"
  }
}
```

**Go 语言运行脚本**：

```jsonc
/* actions/go.json */
{
  "run main": {
    "title": "运行 main",
    "filter": "[[ \"$TM_CURRENT_LINE\" == *main* ]] && echo true || echo false",
    "shell": [
      "alacritty --hold --working-directory ${TM_DIRECTORY} -e go run ${TM_FILENAME};",
      "notify-send \"Golang\" \"RUN: ${TM_FILENAME}\""
    ],
    "description": "在新终端中运行 Go 主程序"
  },
  "run main in tmux": {
    "title": "tmux: 运行 main",
    "filter": "[[ \"$(cat)\" == *main* ]] && echo true || echo false",
    "shell": [
      "tmux split-window -h -c ${WORKSPACE_FOLDER}; tmux send 'go run ${TM_FILENAME}' Enter"
    ],
    "description": "在 tmux 中运行 Go 主程序"
  }
}
```

---

## 变量（Variables）

变量可以在 `snippet.body`、`action.filter` 和 `action.shell` 中使用。

> **语法**：支持 `$VARIABLE` 和 `${VARIABLE}` 两种格式。

### 路径相关

| 变量 | 说明 |
|------|------|
| `TM_SELECTED_TEXT` | 当前选中的文本 |
| `TM_CURRENT_LINE` | 光标所在行的内容 |
| `TM_CURRENT_WORD` | 光标所在的单词 |
| `TM_LINE_INDEX` | 光标所在行（0 开始索引） |
| `TM_LINE_NUMBER` | 光标所在行（1 开始索引） |
| `TM_FILENAME` | 当前文件名 |
| `TM_FILENAME_BASE` | 当前文件名（不含扩展名） |
| `TM_DIRECTORY` | 当前文件所在目录 |
| `TM_FILEPATH` | 当前文件的完整路径 |
| `RELATIVE_FILEPATH` | 相对于工作区的文件路径 |
| `CLIPBOARD` | 剪贴板内容 |
| `WORKSPACE_NAME` | 工作区/文件夹名称 |
| `WORKSPACE_FOLDER` | 工作区/文件夹路径 |
| `CURSOR_INDEX` | 光标索引（0 开始） |
| `CURSOR_NUMBER` | 光标索引（1 开始） |

### 日期时间

| 变量 | 说明 | 示例 |
|------|------|------|
| `CURRENT_YEAR` | 当前年份 | `2025` |
| `CURRENT_YEAR_SHORT` | 年份后两位 | `25` |
| `CURRENT_MONTH` | 月份（补零） | `02` |
| `CURRENT_MONTH_NAME` | 月份全称 | `February` |
| `CURRENT_MONTH_NAME_SHORT` | 月份缩写 | `Feb` |
| `CURRENT_DATE` | 日期（补零） | `08` |
| `CURRENT_DAY_NAME` | 星期全称 | `Saturday` |
| `CURRENT_DAY_NAME_SHORT` | 星期缩写 | `Sat` |
| `CURRENT_HOUR` | 小时（24 小时制） | `14` |
| `CURRENT_MINUTE` | 分钟 | `30` |
| `CURRENT_SECOND` | 秒 | `45` |
| `CURRENT_SECONDS_UNIX` | Unix 时间戳 | `1738930245` |
| `CURRENT_TIMEZONE_OFFSET` | 时区偏移 | `+08:00` |

### 随机值

| 变量 | 说明 |
|------|------|
| `RANDOM` | 6 位随机数字 |
| `RANDOM_HEX` | 6 位随机十六进制字符串 |
| `UUID` | UUID v4 |

### 注释符号（预留）

| 变量 | 说明 |
|------|------|
| `BLOCK_COMMENT_START` | 块注释开始符号 |
| `BLOCK_COMMENT_END` | 块注释结束符号 |
| `LINE_COMMENT` | 行注释符号 |

---

## 文档颜色（DocumentColor）

hx-lsp 支持识别多种颜色格式，并在编辑器中显示颜色预览。

### 标准 CSS 颜色

**十六进制**：
- `#ffffff` - 标准 6 位十六进制

**RGB/RGBA**：
- `rgb(255, 255, 255)` - 整数形式
- `rgb(2.0, 255.0, 255.0)` - 浮点数形式
- `rgb(100%, 0%, 50%)` - 百分比形式
- `rgba(1.0, 0.0, 0.0, 0.5)` - 带透明度

**HSL/HSLA**：
- `hsl(240, 50%, 50%)` - 色相 0-360 度，饱和度/亮度百分比
- `hsl(180, 0.5, 0.5)` - 浮点数形式
- `hsla(300, 100%, 100%, 0.5)` - 带透明度

**HSV/HSVA**：
- `hsv(300, 100%, 100%)` - 色相 0-360 度，饱和度/明度百分比
- `hsv(180, 0.5, 0.5)` - 浮点数形式
- `hsva(180, 0.5, 0.5, 0.5)` - 带透明度

### Bevy 游戏引擎颜色

- `srgb(1.0, 0.0, 0.0)` - 标准 RGB（0.0-1.0）
- `srgba(1.0, 0.0, 0.0, 0.8)` - 带透明度

---

## 参考

- [VSCode Snippets 文档](https://code.visualstudio.com/docs/editor/userdefinedsnippets)
- [Helix 编辑器](https://github.com/helix-editor/helix)
- [Helix LSP 配置 Wiki](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)
