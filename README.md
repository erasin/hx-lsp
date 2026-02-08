# hx-lsp

[中文文档](./README.zh-cn.md)

An LSP tool that provides custom code snippets and Code Actions for [Helix Editor](https://github.com/helix-editor/helix).

---

## Features

### LSP Commands

- `reload snippets` - Reload snippet configurations
- `reload actions` - Reload action configurations

### Core Features

| Feature | Description | Related PR |
|---------|-------------|------------|
| **Completion** | VSCode-style code snippets | [helix#9801](https://github.com/helix-editor/helix/pull/9801) |
| **Code Actions** | Custom shell script actions | - |
| **Document Colors** | CSS/Bevy color preview | [helix#12308](https://github.com/helix-editor/helix/pull/12308) |
| **Word Case Conversion** | snake_case, CamelCase, PascalCase | - |

### Markdown-Specific Features

- **Table Formatter** - Auto-align table columns (selection must contain header separator `|:-`)
- **Text Styling** - Bold, Italic, Strikethrough
- **List Conversion** - Ordered, Unordered, Task lists

---

## Installation

### From crates.io (Recommended)

```bash
cargo install --force hx-lsp
```

### From Source

```bash
git clone https://github.com/erasin/hx-lsp.git
cd hx-lsp
cargo install --path .
```

---

## Configuration

### Helix Language Configuration

Edit Helix's language configuration file `languages.toml`:

- Global: `$XDG_CONFIG_HOME/helix/languages.toml`
- Project: `WORKSPACE_ROOT/.helix/languages.toml`

> **About `WORKSPACE_ROOT`**: Obtained from the `rootPath` in Helix's `initialize` request. When multiple `.helix` directories exist at different levels, the closest one is used.

#### Configuration Example

Add hx-lsp support for Markdown:

```toml
[language-server.hx-lsp]
command = "hx-lsp"

[[language]]
name = "markdown"
language-servers = ["marksman", "markdown-oxide", "hx-lsp"]

# Or enable only specific features
language-servers = [
  "marksman",
  "markdown-oxide",
  { name = "hx-lsp", only-features = ["document-colors"] }
]
```

> **About `language id`**: Refer to [helix/languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml) and [Helix Wiki](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations).

Helix supports filtering LSP features using `only-features` and `except-features`. hx-lsp supports:
- `completion` - Code completion
- `code-action` - Code actions
- `document-colors` - Document colors

---

## Configuration Files

Configuration files use `jsonc` format (supports comments, but no trailing commas).

> **Supported comment styles**: `// ...`, `/* ... */`, `# ...`

### File Loading Paths

**Snippets**:
- Global: `$XDG_CONFIG_HOME/helix/snippets/`
- Project: `WORKSPACE_ROOT/.helix/snippets/`

**Actions**:
- Global: `$XDG_CONFIG_HOME/helix/actions/`
- Project: `WORKSPACE_ROOT/.helix/actions/`

When LSP receives `textDocument/didOpen` request, it automatically loads configuration files for the corresponding language.

> Use Helix command `:lsp-workspace-command` to open the command picker and manually reload snippets or actions.

---

## Snippets

hx-lsp is compatible with [VSCode Snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets) format.

### File Naming Convention

- Global snippets: `{name}.code-snippets`
- Language-specific: `{language_id}.json`

```
snippets/
├── global.code-snippets    # Global snippets
├── html.json              # HTML snippets
└── markdown.json          # Markdown snippets
```

### Snippet Format

| Field | Type | Description |
|-------|------|-------------|
| `prefix` | `String` or `String[]` | Trigger keywords for completion |
| `body` | `String` or `String[]` | Snippet content |
| `description` | `String` or `String[]` | Description (optional) |

### Example

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
    "description": "Show current file path information"
  }
}
```

---

## Code Actions

Actions allow conditional execution of shell scripts and insert the output into the editor.

```
actions/
├── html.json
└── markdown.json
```

### Action Format

| Field | Type | Description |
|-------|------|-------------|
| `title` | `String` | Title displayed in Helix |
| `filter` | `String` or `String[]` | Shell script, action enabled when returning `true`, `1`, or empty |
| `shell` | `String` or `String[]` | Shell script, output replaces selected text |
| `description` | `String` or `String[]` | Description (optional) |

> **Note**: Selected text is passed via `Stdio::piped`. Use `$(cat)` to capture it in scripts, or use variable `$TM_SELECTED_TEXT`.

### Examples

**Markdown text formatting**:

```jsonc
/* actions/markdown.json */
{
  "bold": {
    "title": "Bold",
    "filter": "",
    "shell": ["echo -n \"**${TM_SELECTED_TEXT}**\""],
    "description": "Make selected text bold"
  },
  "italic": {
    "title": "Italic",
    "filter": "",
    "shell": ["echo -n \"_${TM_SELECTED_TEXT}_\""],
    "description": "Make selected text italic"
  }
}
```

**Go language run script**:

```jsonc
/* actions/go.json */
{
  "run main": {
    "title": "Run main",
    "filter": "[[ \"$TM_CURRENT_LINE\" == *main* ]] && echo true || echo false",
    "shell": [
      "alacritty --hold --working-directory ${TM_DIRECTORY} -e go run ${TM_FILENAME};",
      "notify-send \"Golang\" \"RUN: ${TM_FILENAME}\""
    ],
    "description": "Run Go main program in new terminal"
  },
  "run main in tmux": {
    "title": "tmux: Run main",
    "filter": "[[ \"$(cat)\" == *main* ]] && echo true || echo false",
    "shell": [
      "tmux split-window -h -c ${WORKSPACE_FOLDER}; tmux send 'go run ${TM_FILENAME}' Enter"
    ],
    "description": "Run Go main program in tmux"
  }
}
```

---

## Variables

Variables can be used in `snippet.body`, `action.filter`, and `action.shell`.

> **Syntax**: Supports both `$VARIABLE` and `${VARIABLE}` formats.

### Path Related

| Variable | Description |
|----------|-------------|
| `TM_SELECTED_TEXT` | Currently selected text |
| `TM_CURRENT_LINE` | Content of the line where cursor is |
| `TM_CURRENT_WORD` | Word under cursor |
| `TM_LINE_INDEX` | Line index (0-based) |
| `TM_LINE_NUMBER` | Line number (1-based) |
| `TM_FILENAME` | Current filename |
| `TM_FILENAME_BASE` | Current filename without extension |
| `TM_DIRECTORY` | Directory of current file |
| `TM_FILEPATH` | Full path of current file |
| `RELATIVE_FILEPATH` | File path relative to workspace |
| `CLIPBOARD` | Clipboard content |
| `WORKSPACE_NAME` | Workspace/folder name |
| `WORKSPACE_FOLDER` | Workspace/folder path |
| `CURSOR_INDEX` | Cursor index (0-based) |
| `CURSOR_NUMBER` | Cursor index (1-based) |

### Date & Time

| Variable | Description | Example |
|----------|-------------|---------|
| `CURRENT_YEAR` | Current year | `2025` |
| `CURRENT_YEAR_SHORT` | Last two digits of year | `25` |
| `CURRENT_MONTH` | Month (zero-padded) | `02` |
| `CURRENT_MONTH_NAME` | Full month name | `February` |
| `CURRENT_MONTH_NAME_SHORT` | Short month name | `Feb` |
| `CURRENT_DATE` | Date (zero-padded) | `08` |
| `CURRENT_DAY_NAME` | Full day name | `Saturday` |
| `CURRENT_DAY_NAME_SHORT` | Short day name | `Sat` |
| `CURRENT_HOUR` | Hour (24-hour format) | `14` |
| `CURRENT_MINUTE` | Minute | `30` |
| `CURRENT_SECOND` | Second | `45` |
| `CURRENT_SECONDS_UNIX` | Unix timestamp | `1738930245` |
| `CURRENT_TIMEZONE_OFFSET` | Timezone offset | `+08:00` |

### Random Values

| Variable | Description |
|----------|-------------|
| `RANDOM` | 6-digit random number |
| `RANDOM_HEX` | 6-digit random hex string |
| `UUID` | UUID v4 |

### Comment Symbols (Reserved)

| Variable | Description |
|----------|-------------|
| `BLOCK_COMMENT_START` | Block comment start symbol |
| `BLOCK_COMMENT_END` | Block comment end symbol |
| `LINE_COMMENT` | Line comment symbol |

---

## Document Color

hx-lsp supports recognizing various color formats and displaying color previews in the editor.

### Standard CSS Colors

**Hexadecimal**:
- `#ffffff` - Standard 6-digit hex

**RGB/RGBA**:
- `rgb(255, 255, 255)` - Integer format
- `rgb(2.0, 255.0, 255.0)` - Float format
- `rgb(100%, 0%, 50%)` - Percentage format
- `rgba(1.0, 0.0, 0.0, 0.5)` - With alpha

**HSL/HSLA**:
- `hsl(240, 50%, 50%)` - Hue 0-360 degrees, saturation/lightness percentages
- `hsl(180, 0.5, 0.5)` - Float format
- `hsla(300, 100%, 100%, 0.5)` - With alpha

**HSV/HSVA**:
- `hsv(300, 100%, 100%)` - Hue 0-360 degrees, saturation/value percentages
- `hsv(180, 0.5, 0.5)` - Float format
- `hsva(180, 0.5, 0.5, 0.5)` - With alpha

### Bevy Game Engine Colors

- `srgb(1.0, 0.0, 0.0)` - Standard RGB (0.0-1.0)
- `srgba(1.0, 0.0, 0.0, 0.8)` - With alpha

---

## References

- [VSCode Snippets Documentation](https://code.visualstudio.com/docs/editor/userdefinedsnippets)
- [Helix Editor](https://github.com/helix-editor/helix)
- [Helix LSP Configuration Wiki](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations)
