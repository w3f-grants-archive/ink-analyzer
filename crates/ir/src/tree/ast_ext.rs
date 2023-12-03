//! AST traversal utilities.

use ra_ap_syntax::ast::{HasAttrs, HasName};
use ra_ap_syntax::{ast, AstNode, SyntaxKind, SyntaxNode, SyntaxToken};
use std::collections::{HashMap, HashSet};

use crate::traits::IsSyntax;

/// Returns the closest AST ancestor of a specific type for the syntax "element".
pub fn closest_ancestor_ast_type<I, T>(item: &I) -> Option<T>
where
    I: IsSyntax,
    T: AstNode,
{
    let parent = item.parent()?;
    if T::can_cast(parent.kind()) {
        T::cast(parent)
    } else {
        closest_ancestor_ast_type(&parent)
    }
}

/// Returns parent [AST Item](https://github.com/rust-lang/rust-analyzer/blob/master/crates/syntax/src/ast/generated/nodes.rs#L1589-L1610)
/// for the syntax "element".
pub fn parent_ast_item<T>(node: &T) -> Option<ast::Item>
where
    T: IsSyntax,
{
    closest_ancestor_ast_type::<T, ast::Item>(node).and_then(|item| {
        if node.kind() == SyntaxKind::ATTR {
            // If the subject is an attribute, we make sure it's actually applied to the AST item.
            // This handles the case where an attribute is not really applied to any AST item.
            item.attrs()
                .any(|attr| {
                    attr.syntax().kind() == node.kind()
                        && attr.syntax().text_range() == node.text_range()
                })
                .then_some(item)
        } else {
            Some(item)
        }
    })
}

/// Returns the closest non-trivia token based on the step expression.
pub fn closest_non_trivia_token<F>(token: &SyntaxToken, step_expr: F) -> Option<SyntaxToken>
where
    F: Fn(&SyntaxToken) -> Option<SyntaxToken>,
{
    closest_item_which(
        token,
        step_expr,
        |subject| !subject.kind().is_trivia(),
        |subject| !subject.kind().is_trivia(),
    )
}

/// Returns the closest non-trivia token based on the input predicates.
pub fn closest_item_which<T, S, G, H>(
    token: &T,
    step_expr: S,
    goal_expr: G,
    halt_expr: H,
) -> Option<T>
where
    S: Fn(&T) -> Option<T>,
    G: Fn(&T) -> bool,
    H: Fn(&T) -> bool,
{
    (step_expr)(token).and_then(|subject| {
        if goal_expr(&subject) {
            Some(subject)
        } else if halt_expr(&subject) {
            None
        } else {
            closest_item_which(&subject, step_expr, goal_expr, halt_expr)
        }
    })
}

/// Determines an item's path based on use statements in the current scope.
#[macro_export]
macro_rules! resolve_item_path_from_use_scope {
    ($name: ident, $root_node: expr) => {{
        let (use_paths, use_aliases) = $crate::simple_use_paths_and_aliases_in_scope($root_node);

        std::iter::once(use_aliases.get(&$name).cloned())
            .flatten()
            .chain(use_paths.into_iter().filter_map(|use_path| {
                let path_str = use_path.replace(' ', "");
                if path_str.ends_with(&format!("::{}", $name)) {
                    Some(path_str)
                } else if let Some(prefix) = path_str.strip_suffix("::*") {
                    Some(format!("{prefix}::{}", $name))
                } else {
                    None
                }
            }))
            .filter_map(|path_str| $crate::path_from_str(&path_str))
            .filter(|path| path.qualifier().is_some())
    }};
}

/// Returns the AST item referenced by the path (if any).
pub fn resolve_item<T>(path: &ast::Path, ref_node: &SyntaxNode) -> Option<T>
where
    T: AstNode + HasName,
{
    // Only continue if the last segment is valid.
    let target = path.segment()?;

    match path.qualifier() {
        // Determines the root node/module for target item resolution based on qualifier.
        Some(qualifier) => resolve_qualifier(&qualifier, ref_node, Some(&target)),
        // Otherwise defaults to the current module (if there's no qualifier).
        None => resolve_current_module(ref_node),
    }
    .as_ref()
    .map(resolve_item_list_root)
    .zip(target.name_ref())
    .and_then(|(root_node, target_name)| {
        let resolve_child = || {
            root_node
                .children()
                .filter(|node| T::can_cast(node.kind()))
                .find_map(|node| {
                    T::cast(node).filter(|item| {
                        item.name()
                            .map_or(false, |item_name| item_name.text() == target_name.text())
                    })
                })
        };
        let resolve_from_use_scope = || {
            let item_name = target_name.to_string();
            resolve_item_path_from_use_scope!(item_name, &root_node)
                .find_map(|path| resolve_item(&path, &root_node))
        };

        resolve_child().or(resolve_from_use_scope())
    })
}

/// Resolves current module (defaults to the file root if there's no `mod` item).
pub fn resolve_current_module(node: &SyntaxNode) -> Option<SyntaxNode> {
    ast::Module::can_cast(node.kind())
        .then_some(node.clone())
        .or(node
            .ancestors()
            .find(|it| ast::Module::can_cast(it.kind()))
            .or(node.ancestors().last()))
}

/// Resolves qualifier root/module (if it exists).
pub fn resolve_qualifier(
    path: &ast::Path,
    ref_node: &SyntaxNode,
    target_option: Option<&ast::PathSegment>,
) -> Option<SyntaxNode> {
    // Resolves next child module.
    let resolve_next_child_module = |root: &SyntaxNode, name: &ast::NameRef| {
        let resolve_child = || {
            root.children().find(|it| {
                ast::Module::can_cast(it.kind())
                    && ast::Module::cast(it.clone())
                        .and_then(|module| module.name())
                        .map_or(false, |module_name| module_name.text() == name.text())
            })
        };
        let resolve_from_use_scope = || {
            let item_name = name.to_string();
            resolve_item_path_from_use_scope!(item_name, root)
                .find_map(|path| resolve_qualifier(&path, root, None))
        };
        resolve_child().or(resolve_from_use_scope())
    };

    let mut path_segments = path
        .segments()
        // Calling segments on the qualifier appears to also include the target for some reason,
        // so we filter it out manually.
        .filter(|segment| target_option.map_or(true, |target| segment != target));

    // Resolves first path segment including respecting valid path qualifiers
    // (i.e. `::`, `crate`, `self`, `super`).
    // NOTE: $crate and Self aren't valid path qualifiers for our context
    // so they're are treated as module/item names.
    // Ref: <https://doc.rust-lang.org/reference/paths.html#paths-in-expressions>.
    let mut resolution_root_option = path_segments.next().and_then(|root_segment| {
        if root_segment.coloncolon_token().is_some() || root_segment.crate_token().is_some() {
            // Resolve from crate root (and next path segment if any).
            ref_node.ancestors().last().and_then(|crate_root| {
                match root_segment.coloncolon_token() {
                    // Resolves next segment if path has `::` qualifier.
                    Some(_) => root_segment
                        .name_ref()
                        .and_then(|name| resolve_next_child_module(&crate_root, &name)),
                    // Otherwise returns the crate root.
                    None => Some(crate_root),
                }
            })
        } else if root_segment.self_token().is_some() {
            // Resolve from current module.
            resolve_current_module(ref_node)
        } else if root_segment.super_token().is_some() {
            // Resolve from parent module.
            resolve_current_module(ref_node)
                .as_ref()
                .and_then(SyntaxNode::parent)
                .as_ref()
                .and_then(resolve_current_module)
        } else {
            resolve_current_module(ref_node)
                .zip(root_segment.name_ref())
                .and_then(|(current_module, name)| {
                    resolve_next_child_module(&current_module, &name)
                })
        }
    });

    // Resolves the remaining qualifier segments (if any).
    while let Some((node, segment)) = resolution_root_option.as_ref().zip(path_segments.next()) {
        resolution_root_option = segment
            .name_ref()
            .and_then(|name| resolve_next_child_module(node, &name));
    }

    resolution_root_option
}

/// Returns all use-paths and aliases in the current scope as flattened as simple paths.
pub fn simple_use_paths_and_aliases_in_scope(
    ref_node: &SyntaxNode,
) -> (HashSet<String>, HashMap<String, String>) {
    let mut use_paths = HashSet::new();
    let mut use_aliases = HashMap::new(); // alias -> use-path

    let use_results = resolve_item_list_root(ref_node)
        .children()
        .filter_map(|node| ast::Use::cast(node).as_ref().and_then(ast::Use::use_tree))
        .flat_map(|use_tree| flatten_use_tree(&use_tree));

    for (use_path, alias_option) in use_results {
        let use_path = use_path.replace(' ', "");
        match alias_option {
            None => {
                use_paths.insert(use_path);
            }
            Some(alias) => {
                use_aliases.insert(alias, use_path);
            }
        }
    }

    (use_paths, use_aliases)
}

/// Converts a string to a path (if possible).
pub fn path_from_str(path_str: &str) -> Option<ast::Path> {
    ra_ap_syntax::hacks::parse_expr_from_str(path_str).and_then(|expr| match expr {
        ast::Expr::PathExpr(path_expr) => path_expr.path(),
        _ => None,
    })
}

fn resolve_item_list_root(node: &SyntaxNode) -> SyntaxNode {
    ast::Item::cast(node.clone())
        .and_then(|item| match item {
            // We only care about item lists that can contain use statements (for now).
            ast::Item::Module(it) => it.item_list().as_ref().map(AstNode::syntax).cloned(),
            ast::Item::Fn(it) => it
                .body()
                .as_ref()
                .and_then(ast::BlockExpr::stmt_list)
                .as_ref()
                .map(AstNode::syntax)
                .cloned(),
            _ => None,
        })
        // Defaults to supplied node.
        .unwrap_or(node.clone())
}

// "Flattens" a "use tree" into a list of simple paths.
// NOTE: Conceptually, this transforms `use a::{b, c as d};` into [`use a::b;`, `use a::c as d;`].
fn flatten_use_tree(use_tree: &ast::UseTree) -> Vec<(String, Option<String>)> {
    let alias = use_tree.rename().and_then(|rename| {
        rename
            .name()
            .as_ref()
            .map(ToString::to_string)
            .or(rename.underscore_token().as_ref().map(ToString::to_string))
    });
    let add_prefix = |sub_paths: Vec<(String, Option<String>)>| match use_tree.path() {
        None => sub_paths,
        Some(path_prefix) => sub_paths
            .into_iter()
            .map(|(sub_path, alias)| (format!("{path_prefix}::{sub_path}"), alias))
            .collect(),
    };
    if let Some(use_tree_list) = use_tree.use_tree_list() {
        add_prefix(
            use_tree_list
                .use_trees()
                .flat_map(|it| flatten_use_tree(&it))
                .collect(),
        )
    } else if use_tree.star_token().is_some() {
        add_prefix(vec![(String::from("*"), alias)])
    } else if let Some(path) = use_tree.path() {
        vec![(path.to_string().replace(' ', ""), alias)]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use quote::quote;
    use test_utils::quote_as_str;

    #[test]
    fn parent_ast_item_works() {
        let code = quote! {
            #[ink::contract]
            mod my_contract {
                #[ink(event)]
                pub struct MyEvent {
                    #[ink(topic)]
                    field_1: i32,
                    field_2: bool,
                }
            }
        };

        let module = parse_first_ast_node_of_type::<ast::Module>(quote_as_str! { #code });
        let struct_item = parse_first_ast_node_of_type::<ast::Struct>(quote_as_str! { #code });
        let field = parse_first_ast_node_of_type::<ast::RecordField>(quote_as_str! { #code });

        // struct is the AST parent of the field.
        assert_eq!(
            parent_ast_item(field.syntax())
                .unwrap()
                .syntax()
                .text_range(),
            struct_item.syntax().text_range()
        );

        // module is the AST parent of the struct.
        assert_eq!(
            parent_ast_item(struct_item.syntax())
                .unwrap()
                .syntax()
                .text_range(),
            module.syntax().text_range()
        );
    }
}
