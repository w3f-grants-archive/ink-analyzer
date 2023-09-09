//! A text edit.

use ink_analyzer_ir::syntax::{AstNode, SyntaxKind, SyntaxToken, TextRange, TextSize};
use ink_analyzer_ir::{ast, FromSyntax, InkFile};
use once_cell::sync::Lazy;
use regex::Regex;

use super::utils;

/// A text edit (with an optional snippet - i.e tab stops and/or placeholders).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextEdit {
    /// Replacement text for the text edit.
    pub text: String,
    /// Range to which the text edit will be applied.
    pub range: TextRange,
    /// Formatted snippet for the text edit (includes tab stops and/or placeholders).
    pub snippet: Option<String>,
}

impl TextEdit {
    /// Creates text edit.
    pub fn new(text: String, range: TextRange, snippet: Option<String>) -> Self {
        Self {
            text,
            range,
            snippet,
        }
    }

    /// Creates text edit for inserting at the given offset.
    pub fn insert(text: String, offset: TextSize) -> Self {
        Self::insert_with_snippet(text, offset, None)
    }

    /// Creates text edit for inserting at the given offset (including an optional snippet).
    pub fn insert_with_snippet(text: String, offset: TextSize, snippet: Option<String>) -> Self {
        Self {
            text,
            range: TextRange::new(offset, offset),
            snippet,
        }
    }

    /// Creates text edit for replacing the given range.
    pub fn replace(text: String, range: TextRange) -> Self {
        Self::replace_with_snippet(text, range, None)
    }

    /// Creates text edit for replacing the given range (including an optional snippet) - i.e an alias of [`Self::new`].
    pub fn replace_with_snippet(text: String, range: TextRange, snippet: Option<String>) -> Self {
        Self::new(text, range, snippet)
    }

    /// Creates a text edit for deleting the specified range.
    pub fn delete(range: TextRange) -> Self {
        Self {
            text: "".to_string(),
            range,
            snippet: None,
        }
    }
}

/// Format text edits (i.e. add indenting and new lines based on context).
pub fn format_edits(edits: Vec<TextEdit>, file: &InkFile) -> impl Iterator<Item = TextEdit> + '_ {
    edits.into_iter().map(|item| format_edit(item, file))
}

/// Format text edit (i.e. add indenting and new lines based on context).
pub fn format_edit(mut edit: TextEdit, file: &InkFile) -> TextEdit {
    // Only format inserts and replaces (ignore deletes).
    if !edit.text.is_empty() {
        // Determines the token right before the start of the edit offset.
        let token_before_option = file
            .syntax()
            .token_at_offset(edit.range.start())
            .left_biased()
            .filter(|it| it.text_range().end() <= edit.range.start());
        // Determines the token right after the end of the edit offset.
        let token_after_option = file
            .syntax()
            .token_at_offset(edit.range.end())
            .right_biased()
            .filter(|it| it.text_range().start() >= edit.range.end());

        if let Some(token_before) = token_before_option {
            let (prefix, suffix) = match token_before.kind() {
                // Handles edits after whitespace.
                SyntaxKind::WHITESPACE => {
                    (
                        // No formatting prefix.
                        None,
                        // Adds formatting suffix only if the edit is not surrounded by whitespace (treats end of the file like whitespace)
                        // and its preceding whitespace contains a new line but doesn't end with a new line.
                        (token_after_option.as_ref().map_or(false, |token_after| {
                            token_after.kind() != SyntaxKind::WHITESPACE
                        }) && token_before.text().contains('\n')
                            && !token_before.text().ends_with('\n'))
                        .then_some(format!("\n{}", utils::end_indenting(token_before.text()),)),
                    )
                }
                // Handles edits at the beginning of blocks (i.e right after the opening curly bracket).
                SyntaxKind::L_CURLY => {
                    (
                        // Adds formatting prefix only if the edit doesn't start with a new line
                        // and then only add indenting if the edit doesn't start with a space (i.e ' ') or a tab (i.e. '\t').
                        (!edit.text.starts_with('\n')).then(|| {
                            format!(
                                "\n{}",
                                (!edit.text.starts_with(' ') && !edit.text.starts_with('\t'))
                                    .then(|| {
                                        ink_analyzer_ir::closest_ancestor_ast_type::<
                                            SyntaxToken,
                                            ast::Item,
                                        >(&token_before)
                                        .map(|it| utils::item_children_indenting(it.syntax()))
                                    })
                                    .flatten()
                                    .as_deref()
                                    .unwrap_or_default()
                            )
                        }),
                        // Adds formatting suffix if the edit is followed by either a non-whitespace character
                        // or whitespace that doesn't start with at least 2 new lines (the new lines can be interspersed with other whitespace)
                        // and the edit doesn't end with 2 new lines.
                        token_after_option.as_ref().and_then(|token_after| {
                            ((token_after.kind() != SyntaxKind::WHITESPACE
                                || !starts_with_two_or_more_newlines(token_after.text()))
                                && !edit.text.ends_with("\n\n"))
                            .then_some(format!(
                                "\n{}",
                                if token_after.text().starts_with('\n') {
                                    ""
                                } else {
                                    "\n"
                                }
                            ))
                        }),
                    )
                }
                // Handles edits at the end a statement or block.
                SyntaxKind::SEMICOLON | SyntaxKind::R_CURLY => {
                    (
                        // Adds formatting prefix only if the edit doesn't start with a new line
                        // and then only add indenting if the edit doesn't start with a space (i.e ' ') or a tab (i.e. '\t').
                        (!edit.text.starts_with('\n')).then(|| {
                            format!(
                                "\n\n{}",
                                (!edit.text.starts_with(' ') && !edit.text.starts_with('\t'))
                                    .then(|| {
                                        ink_analyzer_ir::closest_ancestor_ast_type::<
                                            SyntaxToken,
                                            ast::Item,
                                        >(&token_before)
                                        .and_then(|it| utils::item_indenting(it.syntax()))
                                    })
                                    .flatten()
                                    .as_deref()
                                    .unwrap_or_default()
                            )
                        }),
                        // No formatting suffix.
                        None,
                    )
                }
                // Ignores all other cases.
                _ => (None, None),
            };

            // Adds formatting if necessary.
            if prefix.is_some() || suffix.is_some() {
                edit.text = format!(
                    "{}{}{}",
                    prefix.as_deref().unwrap_or_default(),
                    edit.text,
                    suffix.as_deref().unwrap_or_default(),
                );
                edit.snippet = edit.snippet.map(|snippet| {
                    format!(
                        "{}{snippet}{}",
                        prefix.as_deref().unwrap_or_default(),
                        suffix.as_deref().unwrap_or_default()
                    )
                });
            }
        }
    }
    edit
}

/// Checks whether the given text starts with at least 2 new lines (the new lines can be interspersed with other whitespace).
fn starts_with_two_or_more_newlines(text: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([^\S\n]*\n[^\S\n]*){2,}").unwrap());
    RE.is_match(text)
}