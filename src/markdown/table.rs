use async_lsp::lsp_types::{Position, Range, TextEdit};
use comrak::{
    Arena, ComrakOptions, ExtensionOptions,
    nodes::{AstNode, NodeValue, TableAlignment},
    parse_document,
};
use ropey::RopeSlice;
use tracing::info;
use unicode_width::UnicodeWidthStr;

/// 格式化 Markdown 表格
pub fn format(rope: RopeSlice, range: Range) -> Vec<TextEdit> {
    let tables = parse_tables(rope, range.start);
    tables
        .iter()
        .map(
            |Table {
                 header,
                 alignments,
                 rows,
                 range,
                 col_widths,
             }| {
                let separator = gen_separator(alignments, col_widths);
                let rows: Vec<String> = [header.clone(), separator]
                    .iter()
                    .chain(rows.iter())
                    .map(|row| format_row(row, col_widths, alignments))
                    .collect();

                let new_text = rows.join("\n");

                TextEdit {
                    range: *range,
                    new_text,
                }
            },
        )
        .collect()
}

#[derive(Clone, Debug, Default)]
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    alignments: Vec<TableAlignment>,
    col_widths: Vec<usize>,
    range: Range,
}

/// 解析表格内容
fn parse_tables(rope: RopeSlice, start_line: Position) -> Vec<Table> {
    let arena = Arena::new();
    let options = ComrakOptions {
        extension: ExtensionOptions {
            table: true,
            strikethrough: false,
            tagfilter: false,
            autolink: false,
            tasklist: false,
            superscript: false,
            header_ids: None,
            footnotes: false,
            description_lists: false,
            front_matter_delimiter: None,
            multiline_block_quotes: false,
            alerts: false,
            math_dollars: false,
            math_code: false,
            wikilinks_title_after_pipe: false,
            wikilinks_title_before_pipe: false,
            underline: false,
            subscript: false,
            spoiler: false,
            greentext: false,
            image_url_rewriter: None,
            link_url_rewriter: None,
        },
        ..Default::default()
    };

    let root = parse_document(&arena, &rope.to_string(), &options);

    let tables = find_table_nodes(root);

    tables
        .iter()
        .filter_map(|(alignments, table_node)| {
            // 获取表格在原始文档中的行范围
            let table_range = get_table_range(table_node, start_line)?;

            let mut header: Vec<String> = Vec::new();
            let mut rows: Vec<Vec<String>> = Vec::new();

            // 遍历表格的子节点
            for node in table_node.children() {
                match &node.data.borrow().value {
                    // 表头
                    NodeValue::TableRow(true) if header.is_empty() => {
                        header = extract_row_cells(node, rope);
                    }
                    // 其他 TableRow 是数据行
                    NodeValue::TableRow(false) => {
                        rows.push(extract_row_cells(node, rope));
                    }
                    _ => {}
                }
            }

            if alignments.is_empty() {
                return None;
            }

            let col_widths = calculate_column_widths(&header, &rows, alignments);

            let table = Table {
                header,
                rows,
                alignments: alignments.clone(),
                col_widths,
                range: table_range,
            };

            info!("TABLE: {:?}", table);

            Some(table)
        })
        .collect()
}

/// 查找表格节点
fn find_table_nodes<'a>(root: &'a AstNode<'a>) -> Vec<(Vec<TableAlignment>, &'a AstNode<'a>)> {
    let mut tables = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let NodeValue::Table(table) = &node.data.borrow().value {
            tables.push((table.alignments.clone(), node))
        }
        stack.extend(node.children());
    }
    tables
}

/// 获取表格在文档中的行范围
fn get_table_range(table_node: &AstNode, start: Position) -> Option<Range> {
    let pos = table_node.data.borrow().sourcepos;

    Some(Range {
        start: Position {
            line: pos.start.line as u32 - 1 + start.line,
            character: pos.start.column as u32 - 1 + start.character,
        },
        end: Position {
            line: pos.end.line as u32 - 1 + start.line,
            character: pos.end.column as u32 + start.character,
        },
    })
}

/// 提取行中的单元格
fn extract_row_cells<'a>(row_node: &'a AstNode<'a>, rope: RopeSlice) -> Vec<String> {
    let mut cells = Vec::new();

    for cell in row_node.children() {
        if let NodeValue::TableCell = cell.data.borrow().value {
            let sourcepos = cell.data.borrow().sourcepos;

            let start_byte = sourcepos.start.column - 1;
            let end_byte = sourcepos.end.column;

            // 从 RopeSlice 中提取单元格文本
            if let Some(slice) = rope
                .line(sourcepos.start.line - 1)
                .get_slice(start_byte..end_byte)
            {
                cells.push(slice.to_string().trim().to_string());
            } else {
                cells.push(String::new());
            }
        }
    }

    cells
}

/// 计算每列的最大宽度
fn calculate_column_widths(
    header: &[String],
    rows: &[Vec<String>],
    alignments: &[TableAlignment],
) -> Vec<usize> {
    let mut widths: Vec<usize> = alignments
        .iter()
        .map(get_alignment_cell_minimum_width)
        .collect();

    rows.iter()
        .cloned()
        .chain(vec![header.to_vec()])
        .for_each(|row| {
            row.iter()
                .enumerate()
                .for_each(|(i, cell)| widths[i] = widths[i].max(cell.width()));
        });

    widths
}

/// 格式化行
fn format_row(cells: &[String], col_widths: &[usize], alignments: &[TableAlignment]) -> String {
    let cells: Vec<String> = cells
        .iter()
        .zip(col_widths)
        .zip(alignments)
        .map(|((cell, width), alignment)| {
            // 根据对齐方式格式化单元格
            match alignment {
                TableAlignment::Left => format!(" {:<width$} ", cell, width = width),
                TableAlignment::Right => format!(" {:>width$} ", cell, width = width),
                TableAlignment::Center => {
                    let cell_width = cell.width();

                    if cell_width >= *width {
                        format!(" {} ", cell,)
                    } else {
                        let left_pad = (width - cell_width) / 2;
                        let right_pad = width - cell_width - left_pad;
                        format!(
                            " {}{}{} ",
                            " ".repeat(left_pad),
                            cell,
                            " ".repeat(right_pad)
                        )
                    }
                }
                _ => format!(" {:<width$} ", cell, width = width), // 默认左对齐
            }
        })
        .collect();

    format!("|{}|", cells.join("|"))
}

/// 格式化分隔线
fn gen_separator(alignments: &[TableAlignment], col_widths: &[usize]) -> Vec<String> {
    alignments
        .iter()
        .zip(col_widths)
        .map(|(&alignment, &width)| {
            let min = get_alignment_cell_minimum_width(&alignment);
            let width = width.max(min);
            match alignment {
                TableAlignment::Left => format!(":{}", "-".repeat(width - 1)),
                TableAlignment::Right => format!("{}:", "-".repeat(width - 1)),
                TableAlignment::Center => format!(":{}:", "-".repeat(width - 2)),
                TableAlignment::None => "-".repeat(width),
            }
        })
        .collect()
}

fn get_alignment_cell_minimum_width(alignment: &TableAlignment) -> usize {
    match alignment {
        TableAlignment::Center => 5,
        TableAlignment::Left | TableAlignment::Right => 4,
        TableAlignment::None => 3,
    }
}
