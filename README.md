# hx-lsp

[![中文文档](https://img.shields.io/badge/lang-zh_CN-red.svg) 中文文档](./README.zh-cn.md)

An LSP tool that provides custom code snippets and Code Actions for [Helix Editor](https://github.com/helix-editor/helix).

## features

- Completion: snippets
- CodeAction: actions

## Install

```sh
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

## Use

Modify the language configuration file `languages.toml` for Helix Editor. 

- `$XDG_CONFIG_HOME/helix/languages.toml`: Helix Configuration file.
- `WORKSPACE_ROOT/.helix/languages.toml` : Configuration file under project workspace root.

Example, Add support for markdown.

```toml
[language-server.hx-lsp]
command = "hx-lsp"

[[language]]
name = "markdown"
language-servers = [ "marksman", "markdown-oxide", "hx-lsp" ]
```

> About `language id`, Read [helix/languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) and [helix wiki language server configurations](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)。

## Configuration

The Configuration file supports the `jsonc` format.

> Comment style: `// ...`, `/* ... */`, `# ...` 。

Snippets file loading path:

- `$XDG_CONFIG_HOME/helix/snippets/`
- `WORKSPACE_ROOT/.helix/snippets/`

Actions file loading path:

- `$XDG_CONFIG_HOME/helix/actions/`
- `WORKSPACE_ROOT/.helix/actions/`

In LSP `textDocument/didOpen` request, The Configuration file with name that is `language_id.json` will be loading.

> Unsupported Dynamic loading config. If you modify configuration file, use `:lsp-restart` to restart lsp and reload the file. 


## Completion: snippets

Code Snippets support [vscode snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) format. The same file suffix supports global suffixes such as`. code-snippets` and language pack suffixes such as`. json`.

For better use snippet completion, Use helix master and merge [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801) to support smart-tab。

```svgbob
.
└── snippets
    ├── global.code-snippets
    ├── html.json
    └── markdown.json
```

Snippet Format：

- **name**: `String`, index
- **prefix**: `String` Or `Vec<String>`, editor completion item
- **body**: `String` Or `Vec<String>`, snippet connent
- **description**: `Option<String>`, Tip

```jsonc
{
  "markdown a": {
    // name
    "prefix": "mda", // string
    "body": "mda in .helix: ${1:abc} : ${2:cde}", // string
    "description": "test a info content in .helix",
  },
  "markdown b": {
    "prefix": [
      // array
      "mdb",
    ],
    "body": "mdb: ${1:abc} : ${2:cde}", // string
    "description": "test b info content",
  },
  "markdown c": {
    "prefix": [
      // array
      "mdc",
      "mdd",
    ],
    "body": [
      // array
      "mda: ${1:abc} : ${2:cde}",
      "test",
    ],
    "description": "test c,d info content",
  },
}
```

## CodeAction: actions

```svgbob
.
└── actions
    ├── html.json
    └── markdown.json
```

Snippet Formatter：

- **title**: `String` helix editor show Code Action Item
- **catch**: `String` catch line content，regex ，code action
- **shell**: `String` Or `Vec<String>` , take shell script
- **description**: `Option<String>` Tip content

```jsonc
/* actions/go.json */
{
  "run main": {
    "title": "go run main",
    "catch": "func main",
    "shell": [
      "alacritty --working-directory ${TM_DIRECTORY} -e go run ${TM_FILENAME}"
    ],
    "description": "go run main"
  }
}
```

```jsonc
/* test */
{
  "tmux split window helix": {
    "title": "tmux split window in project",
    "catch": "fn",
    "shell": [
      "tmux split-window -h",
      "tmux send cd ${WORKSPACE_FOLDER}"
    ],
    "description": "tmux split and open helix in project"
  }
}
```

**catch**：

- [x] regex line
- [ ] selected content
- [ ] match in regex


## Variables

Support variable for snippet body and action shell.

Support like `$UUID` 和 `${UUID}`。

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
