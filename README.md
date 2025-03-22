# hx-lsp

[![中文文档](https://img.shields.io/badge/lang-zh_CN-red.svg) 中文文档](./README.zh-cn.md)

An LSP tool that provides custom code snippets and Code Actions for [Helix Editor](https://github.com/helix-editor/helix).

## Features

- Completion: snippets
- CodeAction: actions (helix#9801)
- Document Color (helix#12308)

## Install

**From crate**

```sh
cargo install --force hx-lsp
```

**From source**

```sh
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

## Use

Modify the language configuration file `languages.toml` for Helix Editor. 

- `$XDG_CONFIG_HOME/helix/languages.toml`: Helix Configuration file.
- `WORKSPACE_ROOT/.helix/languages.toml` : Configuration file under project workspace root.

> About 'WORKSPACE_ROOT',  It is read the 'rootPath' from the 'initialize' provided by Helix, when there are multiple levels of rootPath(`language.roots` of languages.toml), It will read the closest of root '.helix'.

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

~~For better use snippet completion, Use helix master and merge [helix#9081 Add a snippet system](https://github.com/helix-editor/helix/pull/9801) to support smart-tab。 ~~

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
```

Snippet Formatter：

- **title**: `String` helix editor show Code Action Item
- **flter**: `String` Or `Vec<String>` Shell script: return `true`,`1` or empty , 
- **shell**: `String` Or `Vec<String>` Shell script: take shell script
- **description**: `Option<String>` Tip content

```jsonc
/* actions/markdown.json */
{
	"bold": {
		"title": "bold",
		"filter": "",
		"shell": ["echo -n **${TM_SELECTED_TEXT}**"],
		"description": "bold"
	},
	"italic": {
		"title": "italic",
		"filter": "",
		"shell": ["echo -n _${TM_SELECTED_TEXT}_"],
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



## Variables

Support variable for snippet body and action shell.

> [vscode Variables](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_variables)

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

## DocumentColor 

	- rgb(255, 255, 255) supports integers
	- rgb(2.0, 255.0, 255.0) supports floating-point values
  - rgb(100%, 0%, 50%) supports percentages
- hsl
	- hsl(240, 50%, 50%) hue 0-360 degrees, saturation and lightness in percentages.
  - hsl(180, 0.5, 0.5) floating-point values
- hsv
	- hsv(300, 100%, 100%) hue 0-360 degrees, saturation and value in percentages.
  - hsv(180, 0.5, 0.5) floating-point values

## bevy color

- rgba(1.0, 0.0, 0.0, 0.5)
- srgb(1.0,0.0,0.0)
- srgba(1.0, 0.0, 0.0, 0.8)
- hsla(300, 100%, 100%, 0.5) 
- hsva(180, 0.5, 0.5, 0.5) 
