//! Utilities for ink! analysis.

use either::Either;
use ink_analyzer_ir::ast::HasDocComments;
use ink_analyzer_ir::syntax::{
    AstNode, AstToken, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize,
};
use ink_analyzer_ir::{
    ast, FromAST, FromSyntax, HasParent, InkArgKind, InkArgValueKind, InkArgValueStringKind,
    InkAttribute, InkAttributeKind, InkMacroKind, IsInkEntity,
};

/// Returns valid sibling ink! argument kinds for the given ink! attribute kind.
///
/// (i.e argument kinds that don't conflict with the given ink! attribute kind,
/// e.g for the `contract` attribute macro kind, this would be `env` and `keep_attr`
/// while for the `storage` attribute argument kind, this would be `default`, `payable` and `selector`).
pub fn valid_sibling_ink_args(attr_kind: InkAttributeKind) -> Vec<InkArgKind> {
    match attr_kind {
        // Returns valid sibling args (if any) for ink! attribute macros.
        InkAttributeKind::Macro(macro_kind) => {
            match macro_kind {
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/chain_extension.rs#L188-L197>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L848-L1280>.
                InkMacroKind::ChainExtension => Vec::new(),
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/config.rs#L39-L70>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L111-L199>.
                InkMacroKind::Contract => vec![InkArgKind::Env, InkArgKind::KeepAttr],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/storage_item/config.rs#L36-L59>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L772-L799>.
                InkMacroKind::StorageItem => vec![InkArgKind::Derive],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/ink_test.rs#L27-L30>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L805-L846>.
                InkMacroKind::Test => Vec::new(),
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/trait_def/config.rs#L60-L85>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L597-L643>.
                InkMacroKind::TraitDefinition => vec![InkArgKind::KeepAttr, InkArgKind::Namespace],
                // Ref: <https://github.com/paritytech/ink/blob/v4.2.1/crates/e2e/macro/src/config.rs#L49-L85>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.2.1/crates/e2e/macro/src/lib.rs#L41-L45>.
                InkMacroKind::E2ETest => vec![
                    InkArgKind::AdditionalContracts,
                    InkArgKind::Environment,
                    InkArgKind::KeepAttr,
                ],
                _ => Vec::new(),
            }
        }
        // Returns valid sibling args (if any) for ink! attribute arguments.
        // IR crate already makes sure `arg_kind` is the best match regardless of source code order,
        // See [`ink_analyzer_ir::ink_arg_kind_sort_order`] doc.
        InkAttributeKind::Arg(arg_kind) => {
            match arg_kind {
                // Unambiguous `arg_kind`.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item/storage.rs#L83-L93>.
                InkArgKind::Storage => Vec::new(),
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item/event.rs#L88-L98>.
                InkArgKind::Event => vec![InkArgKind::Anonymous],
                InkArgKind::Anonymous => vec![InkArgKind::Event],
                InkArgKind::Topic => Vec::new(),
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/mod.rs#L301-L315>.
                InkArgKind::Impl => vec![InkArgKind::Namespace],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/constructor.rs#L136-L148>.
                // Ref: <https://github.com/paritytech/ink/blob/master/crates/ink/ir/src/ir/item_impl/constructor.rs#L136-L149>.
                InkArgKind::Constructor => vec![
                    InkArgKind::Default,
                    // NOTE: While ink! docs "claim" that "ink! constructors are always implicitly payable and thus cannot be flagged as such",
                    // Ref: <https://github.com/paritytech/ink/blob/v4.2.0/crates/ink/macro/src/lib.rs#L316-L317>,
                    // the `ink_ir` crate currently accepts `payable` annotations for ink! constructors,
                    // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/constructor.rs#L143>,
                    // so we follow the implementation (not the documentation) and thus allow `payable` annotations for ink! constructors.
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                ],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/message.rs#L182-L194>.
                // Ref: <https://github.com/paritytech/ink/blob/master/crates/ink/ir/src/ir/item_impl/message.rs#L182-L195>.
                InkArgKind::Message => vec![
                    InkArgKind::Default,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                ],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/config.rs#L39-L70>.
                InkArgKind::Env => vec![InkArgKind::KeepAttr],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/chain_extension.rs#L476-L487>.
                InkArgKind::Extension => vec![InkArgKind::HandleStatus],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/storage_item/config.rs#L36-L59>.
                InkArgKind::Derive => Vec::new(),

                // Ambiguous `arg_kind`.
                // `keep_attr` is ambiguous because it can be used with both `contract` and `trait_definition` macros.
                // See `contract`, `trait_definition` and `env` patterns above for references.
                InkArgKind::KeepAttr => vec![InkArgKind::Env, InkArgKind::Namespace],
                // Similar to `keep_attr` above, `namespace` can be used with
                // `trait_definition` macro and `impl` argument.
                // But additionally, it can also be a standalone argument on an `impl` block as long as it's not a trait `impl` block.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/mod.rs#L316-L321>.
                // See `trait_definition` and `impl` patterns above for more references.
                InkArgKind::Namespace => vec![InkArgKind::KeepAttr, InkArgKind::Impl],
                // See `extension` pattern above for references.
                InkArgKind::HandleStatus => vec![InkArgKind::Extension],
                // See `constructor` and `message` patterns above for references.
                InkArgKind::Payable => vec![
                    InkArgKind::Constructor,
                    InkArgKind::Default,
                    InkArgKind::Message,
                    InkArgKind::Selector,
                ],
                InkArgKind::Default => vec![
                    InkArgKind::Constructor,
                    InkArgKind::Message,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                ],
                InkArgKind::Selector => vec![
                    InkArgKind::Constructor,
                    InkArgKind::Default,
                    InkArgKind::Message,
                    InkArgKind::Payable,
                ],
                _ => Vec::new(),
            }
        }
    }
}

/// Returns valid quasi-direct descendant ink! argument kinds for the given ink! attribute kind.
///
/// (i.e argument kinds that are allowed in the scope of the given ink! attribute kind,
/// e.g for the `chain_extension` attribute macro kind, this would be `extension` and `handle_status`
/// while for the `event` attribute argument kind, this would be `topic`).
pub fn valid_quasi_direct_descendant_ink_args(attr_kind: InkAttributeKind) -> Vec<InkArgKind> {
    match attr_kind {
        // Returns valid quasi-direct descendant args (if any) for ink! attribute macros.
        InkAttributeKind::Macro(macro_kind) => {
            match macro_kind {
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/chain_extension.rs#L476-L487>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L848-L1280>.
                InkMacroKind::ChainExtension => {
                    vec![InkArgKind::Extension, InkArgKind::HandleStatus]
                }
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item/mod.rs#L58-L116>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L111-L199>.
                InkMacroKind::Contract => vec![
                    InkArgKind::Anonymous,
                    InkArgKind::Constructor,
                    InkArgKind::Default,
                    InkArgKind::Event,
                    InkArgKind::Impl,
                    InkArgKind::Message,
                    InkArgKind::Namespace,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                    InkArgKind::Storage,
                ],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/trait_def/item/trait_item.rs#L85-L99>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/trait_def/item/mod.rs#L163-L164>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/trait_def/item/mod.rs#L290-L296>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L597-L643>.
                InkMacroKind::TraitDefinition => vec![
                    InkArgKind::Default,
                    InkArgKind::Message,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                ],
                // ink! storage items, ink! tests and ink! e2e tests can't have ink! descendants.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L772-L799>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L805-L846>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.2.1/crates/e2e/macro/src/ir.rs#L37-L48>.
                _ => Vec::new(),
            }
        }
        // Returns valid quasi-direct descendant args (if any) for ink! attribute arguments.
        // IR crate already makes sure `arg_kind` is the best match regardless of source code order,
        // See [`ink_analyzer_ir::ink_arg_kind_sort_order`] doc.
        InkAttributeKind::Arg(arg_kind) => {
            match arg_kind {
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item/event.rs#L132-L139>.
                InkArgKind::Event | InkArgKind::Anonymous => vec![InkArgKind::Topic],
                InkArgKind::Topic => Vec::new(),
                // `env` is used with the `contract` macro while `keep_attr` is ambiguous because
                // it can be used with both `contract` and `trait_definition` macro.
                // See `contract`, `trait_definition` patterns above for references.
                InkArgKind::Env | InkArgKind::KeepAttr => vec![
                    InkArgKind::Anonymous,
                    InkArgKind::Constructor,
                    InkArgKind::Default,
                    InkArgKind::Event,
                    InkArgKind::Impl,
                    InkArgKind::Message,
                    InkArgKind::Namespace,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                    InkArgKind::Storage,
                ],
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/mod.rs#L118-L216>.
                // `impl` can be used on `impl` blocks.
                // `namespace` be used with `trait_definition` macro and `impl` argument.
                // But additionally, `namespace` can also be a standalone argument on an `impl` block as long as it's not a trait `impl` block.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item_impl/mod.rs#L316-L321>.
                // See `trait_definition` patterns above for more `namespace` references.
                InkArgKind::Impl | InkArgKind::Namespace => vec![
                    InkArgKind::Constructor,
                    InkArgKind::Default,
                    InkArgKind::Message,
                    InkArgKind::Payable,
                    InkArgKind::Selector,
                ],
                // All other ink! attribute arguments can't have ink! descendants.
                _ => Vec::new(),
            }
        }
    }
}

/// Returns valid quasi-direct descendant ink! macro kinds for the given ink! attribute kind.
///
/// (i.e macro kinds that are allowed in the scope of the given ink! attribute kind,
/// e.g for the `contract` attribute macro kind, this would be `chain_extension`, `storage_item`, `test` and `trait_definition`.
pub fn valid_quasi_direct_descendant_ink_macros(attr_kind: InkAttributeKind) -> Vec<InkMacroKind> {
    match attr_kind {
        // Returns valid quasi-direct descendant macros (if any) for ink! attribute macros.
        InkAttributeKind::Macro(macro_kind) => {
            match macro_kind {
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L111-L199>.
                InkMacroKind::Contract => vec![
                    InkMacroKind::ChainExtension,
                    InkMacroKind::StorageItem,
                    InkMacroKind::Test,
                    InkMacroKind::TraitDefinition,
                    InkMacroKind::E2ETest,
                ],
                // All other ink! attribute macros can't have ink! macro descendants.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L848-L1280>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L772-L799>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L805-L846>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs#L597-L643>.
                // Ref: <https://github.com/paritytech/ink/blob/v4.2.1/crates/e2e/macro/src/ir.rs#L37-L48>.
                _ => Vec::new(),
            }
        }
        // ink! attribute arguments can't have ink! macro descendants.
        // Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/macro/src/lib.rs>.
        InkAttributeKind::Arg(_) => Vec::new(),
    }
}

/// Returns valid ink! argument kinds for the given syntax kind.
///
/// (i.e argument kinds that can be applied to the given syntax kind,
/// e.g for the `impl` syntax kind, this would be `impl` and `namespace`.
pub fn valid_ink_args_by_syntax_kind(syntax_kind: SyntaxKind) -> Vec<InkArgKind> {
    match syntax_kind {
        // `env` and `keep_attr` can only be applied to a `mod` as siblings of an `ink::contract` macro.
        SyntaxKind::MODULE | SyntaxKind::MOD_KW => Vec::new(),
        // `keep_attr` and `namespace` can only be applied to a `trait` as siblings of an `ink::trait_definition` macro.
        SyntaxKind::TRAIT | SyntaxKind::TRAIT_KW => Vec::new(),
        // `derive` can only be applied to an ADT (`enum`, `struct` or `union`) as a sibling of an `ink::storage_item` macro.
        SyntaxKind::STRUCT | SyntaxKind::STRUCT_KW => vec![
            InkArgKind::Anonymous,
            InkArgKind::Event,
            InkArgKind::Storage,
        ],
        SyntaxKind::ENUM | SyntaxKind::ENUM_KW | SyntaxKind::UNION | SyntaxKind::UNION_KW => {
            Vec::new()
        }
        SyntaxKind::RECORD_FIELD => vec![InkArgKind::Topic],
        SyntaxKind::FN | SyntaxKind::FN_KW => vec![
            InkArgKind::Constructor,
            InkArgKind::Default,
            InkArgKind::Extension,
            InkArgKind::HandleStatus,
            InkArgKind::Message,
            InkArgKind::Payable,
            InkArgKind::Selector,
        ],
        SyntaxKind::IMPL | SyntaxKind::IMPL_KW => vec![InkArgKind::Impl, InkArgKind::Namespace],
        _ => Vec::new(),
    }
}

/// Returns valid ink! macro kinds for the given syntax kind.
///
/// (i.e macro kinds that can be applied to the given syntax kind,
/// e.g for the `module` syntax kind, this would be `contract`.
pub fn valid_ink_macros_by_syntax_kind(syntax_kind: SyntaxKind) -> Vec<InkMacroKind> {
    match syntax_kind {
        SyntaxKind::MODULE | SyntaxKind::MOD_KW => vec![InkMacroKind::Contract],
        SyntaxKind::TRAIT | SyntaxKind::TRAIT_KW => {
            vec![InkMacroKind::ChainExtension, InkMacroKind::TraitDefinition]
        }
        SyntaxKind::ENUM
        | SyntaxKind::ENUM_KW
        | SyntaxKind::STRUCT
        | SyntaxKind::STRUCT_KW
        | SyntaxKind::UNION
        | SyntaxKind::UNION_KW => vec![InkMacroKind::StorageItem],
        SyntaxKind::FN | SyntaxKind::FN_KW => vec![InkMacroKind::Test, InkMacroKind::E2ETest],
        _ => Vec::new(),
    }
}

/// Returns the primary ink! attribute candidate for the syntax node (if any),
/// a boolean flag indicating whether its the first ink! attribute.
///
/// (i.e returns either the first valid ink! attribute macro or the highest ranked ink! attribute argument,
/// see [`ink_analyzer_ir::ink_arg_kind_sort_order`] doc for attribute argument ranking criteria).
pub fn primary_ink_attribute_candidate(
    attrs: impl Iterator<Item = InkAttribute>,
) -> Option<(InkAttribute, bool)> {
    let mut candidates: Vec<(u8, InkAttribute, bool)> = attrs
        .enumerate()
        .filter_map(|(idx, attr)| {
            // Ignore unknown attributes.
            (!matches!(
                attr.kind(),
                InkAttributeKind::Macro(InkMacroKind::Unknown)
                    | InkAttributeKind::Arg(InkArgKind::Unknown)
            ))
            .then_some((
                // Assigns the order of the attribute.
                match attr.kind() {
                    // ink! attribute macros get the highest priority.
                    InkAttributeKind::Macro(_) => 0,
                    // ink! attribute arguments get their priority lowered by 1 to keep macros the highest.
                    InkAttributeKind::Arg(arg_kind) => {
                        ink_analyzer_ir::ink_arg_kind_sort_order(*arg_kind) + 1
                    }
                },
                attr,
                // Tracks whether attribute is the first.
                idx == 0,
            ))
        })
        .collect();
    candidates.sort_by_key(|(order, ..)| *order);
    // Returns the best ranked ink! attribute.
    candidates
        .first()
        .cloned()
        .map(|(_, attr, is_first)| (attr, is_first))
}

/// Suggest primary attribute kinds in case the current one is either incomplete
/// (e.g `anonymous` without `event` or `derive` without `storage_item` attribute macro)
/// or ambiguous (e.g `selector` with neither `constructor` nor `message` or
/// `keep_attr` with neither `contract` nor `trait_definition` attribute macros).
pub fn primary_ink_attribute_kind_suggestions(
    attr_kind: InkAttributeKind,
) -> Vec<InkAttributeKind> {
    match attr_kind {
        InkAttributeKind::Arg(arg_kind) => {
            // Only ink! attribute arguments when set as the primary attribute have
            // the potential to be either incomplete or ambiguous.
            // See respective match pattern in the [`utils::valid_sibling_ink_args`] function for the rationale and references.
            match arg_kind {
                InkArgKind::Anonymous => vec![InkAttributeKind::Arg(InkArgKind::Event)],
                InkArgKind::KeepAttr => vec![
                    InkAttributeKind::Macro(InkMacroKind::Contract),
                    InkAttributeKind::Macro(InkMacroKind::TraitDefinition),
                ],
                InkArgKind::HandleStatus => vec![InkAttributeKind::Arg(InkArgKind::Extension)],
                InkArgKind::Namespace => vec![
                    InkAttributeKind::Macro(InkMacroKind::TraitDefinition),
                    InkAttributeKind::Arg(InkArgKind::Impl),
                ],
                InkArgKind::Payable | InkArgKind::Default | InkArgKind::Selector => vec![
                    InkAttributeKind::Arg(InkArgKind::Constructor),
                    InkAttributeKind::Arg(InkArgKind::Message),
                ],
                // Default
                _ => Vec::new(),
            }
        }
        // ink! attribute macros are always complete and unambiguous on their own.
        InkAttributeKind::Macro(_) => Vec::new(),
    }
}

/// Filters out duplicate ink! arguments from suggestions
/// (i.e ink! arguments that are already applied to the attribute's parent node).
pub fn remove_duplicate_ink_arg_suggestions(
    suggestions: &mut Vec<InkArgKind>,
    attr_parent: &SyntaxNode,
) {
    let already_annotated_ink_args: Vec<InkArgKind> = ink_analyzer_ir::ink_attrs(attr_parent)
        .flat_map(|ink_attr| ink_attr.args().to_owned())
        .map(|ink_arg| *ink_arg.kind())
        .collect();
    // Filters out duplicates.
    suggestions.retain(|arg_kind| !already_annotated_ink_args.contains(arg_kind));
}

/// Filters out duplicate ink! macros from suggestions
/// (i.e ink! macros that are already applied to the attribute's parent node).
pub fn remove_duplicate_ink_macro_suggestions(
    suggestions: &mut Vec<InkMacroKind>,
    attr_parent: &SyntaxNode,
) {
    let already_annotated_ink_macros: Vec<InkMacroKind> = ink_analyzer_ir::ink_attrs(attr_parent)
        .filter_map(|ink_attr| match ink_attr.kind() {
            InkAttributeKind::Macro(macro_kind) => Some(*macro_kind),
            InkAttributeKind::Arg(_) => None,
        })
        .collect();
    // Filters out duplicates.
    suggestions.retain(|arg_kind| !already_annotated_ink_macros.contains(arg_kind));
}

/// Filters out conflicting ink! arguments from suggestions
/// (i.e ink! arguments that aren't valid siblings of the best candidate for primary ink! attribute kind of the parent node).
pub fn remove_conflicting_ink_arg_suggestions(
    suggestions: &mut Vec<InkArgKind>,
    attr_parent: &SyntaxNode,
) {
    // Gets the primary ink! attribute candidate (if any).
    if let Some((primary_ink_attr, ..)) =
        primary_ink_attribute_candidate(ink_analyzer_ir::ink_attrs(attr_parent))
    {
        let valid_siblings = valid_sibling_ink_args(*primary_ink_attr.kind());
        // Filters out invalid siblings.
        suggestions.retain(|arg_kind| valid_siblings.contains(arg_kind));
    }
}

/// Filters out invalid ink! arguments from suggestions based on parent ink! scope.
pub fn remove_invalid_ink_arg_suggestions_for_parent_ink_scope(
    suggestions: &mut Vec<InkArgKind>,
    attr_parent: &SyntaxNode,
) {
    let parent_ink_scope_valid_ink_args: Vec<InkArgKind> =
        ink_analyzer_ir::ink_attrs_closest_ancestors(attr_parent)
            .flat_map(|attr| valid_quasi_direct_descendant_ink_args(*attr.kind()))
            .collect();

    // Filters out invalid arguments for the parent ink! scope (if any).
    if !parent_ink_scope_valid_ink_args.is_empty() {
        suggestions.retain(|arg_kind| {
            parent_ink_scope_valid_ink_args.is_empty()
                || parent_ink_scope_valid_ink_args.contains(arg_kind)
        });
    }
}

/// Filters out invalid ink! macros from suggestions based on parent ink! scope.
pub fn remove_invalid_ink_macro_suggestions_for_parent_ink_scope(
    suggestions: &mut Vec<InkMacroKind>,
    attr_parent: &SyntaxNode,
) {
    let parent_ink_scope_valid_ink_macros: Vec<InkMacroKind> =
        ink_analyzer_ir::ink_attrs_closest_ancestors(attr_parent)
            .flat_map(|attr| valid_quasi_direct_descendant_ink_macros(*attr.kind()))
            .collect();

    // Filters out invalid arguments for the parent ink! scope (if any).
    if !parent_ink_scope_valid_ink_macros.is_empty() {
        suggestions.retain(|macro_kind| {
            parent_ink_scope_valid_ink_macros.is_empty()
                || parent_ink_scope_valid_ink_macros.contains(macro_kind)
        });
    }
}

/// Returns the insertion text and snippet (if appropriate) for ink! attribute argument including
/// the `=` symbol after the ink! attribute argument name if necessary.
///
/// (i.e for `selector`, we return `"selector="` while for `payable`, we simply return `"payable"`)
pub fn ink_arg_insertion_text(
    arg_kind: InkArgKind,
    insert_offset: TextSize,
    parent_node: &SyntaxNode,
) -> (String, Option<String>) {
    // Determines whether or not to insert the `=` symbol after the ink! attribute argument name.
    let insert_equal_token = match InkArgValueKind::from(arg_kind) {
        // No `=` symbol is inserted after ink! attribute arguments that should not have a value.
        InkArgValueKind::None => false,
        // Adds an `=` symbol after the ink! attribute argument name if an `=` symbol is not
        // the next closest non-trivia token after the insertion offset.
        _ => parent_node
            .token_at_offset(insert_offset)
            .right_biased()
            .and_then(|token| {
                // Finds the next non-trivia token.
                let is_next_non_trivia_token = |subject: &SyntaxToken| {
                    subject.text_range().start() >= insert_offset && !subject.kind().is_trivia()
                };
                let next_non_trivia_token = if is_next_non_trivia_token(&token) {
                    Some(token)
                } else {
                    ink_analyzer_ir::closest_item_which(
                        &token,
                        SyntaxToken::next_token,
                        is_next_non_trivia_token,
                        is_next_non_trivia_token,
                    )
                };
                next_non_trivia_token.map(|next_token| match next_token.kind() {
                    SyntaxKind::EQ => false,
                    // Adds an `=` symbol only if the next closest non-trivia token is not an `=` symbol.
                    _ => true,
                })
            })
            // Defaults to adding the `=` symbol if the next closest non-trivia token couldn't be determined.
            .unwrap_or(true),
    };
    let text = format!("{arg_kind}{}", if insert_equal_token { " = " } else { "" });
    // Creates (if appropriate) a snippet with tab stops and/or placeholders where applicable.
    let snippet = insert_equal_token.then_some(format!(
        "{text}{}",
        match InkArgValueKind::from(arg_kind) {
            InkArgValueKind::U32 | InkArgValueKind::U32OrWildcard => "${1:1}",
            InkArgValueKind::String(str_kind) => match str_kind {
                InkArgValueStringKind::Identifier => r#""${1:my_namespace}""#,
                _ => r#""$1""#,
            },
            InkArgValueKind::Bool => "${1:true}",
            InkArgValueKind::Path(_) => "${1:crate::}",
            // Should not be able to get here.
            InkArgValueKind::None => "",
        }
    ));

    (text, snippet)
}

/// Returns the insertion offset and affixes (e.g whitespace to preserve formatting) for an ink! attribute.
pub fn ink_attribute_insertion_offset_and_affixes(
    parent_ast_node: Either<&ast::Item, &ast::RecordField>,
) -> (TextSize, Option<String>, Option<String>) {
    // Retrieves the parent syntax node and it's the last attribute or doc comment (if any).
    let (parent_syntax_node, last_attr_or_doc_comment) = match parent_ast_node {
        Either::Left(ast_item) => (ast_item.syntax(), ast_item.doc_comments_and_attrs().last()),
        Either::Right(record_field) => (
            record_field.syntax(),
            record_field.doc_comments_and_attrs().last(),
        ),
    };

    // Determines the insertion suffix (i.e indenting - so that we preserve formatting) for the ink! attribute.
    // It's always a suffix because we insert at the beginning of the target item's first non-(attribute/rustdoc/trivia) token,
    let get_insert_indenting =
        |node: &SyntaxNode| item_indenting(node).map(|indent| format!("\n{indent}"));

    last_attr_or_doc_comment
        .and_then(|item| match item {
            Either::Left(attr) => ink_analyzer_ir::last_child_token(attr.syntax()),
            Either::Right(comment) => Some(comment.syntax().clone()),
        })
        .or(ink_analyzer_ir::first_child_token(parent_syntax_node)
            .and_then(|first_token| first_token.kind().is_trivia().then_some(first_token)))
        // Finds the first non-(attribute/rustdoc/trivia) token for the AST item.
        .and_then(|it| ink_analyzer_ir::closest_non_trivia_token(&it, SyntaxToken::next_token))
        .map_or(
            (
                parent_syntax_node.text_range().start(),
                None,
                get_insert_indenting(parent_syntax_node),
            ),
            |first_non_attr_or_doc_token| {
                (
                    first_non_attr_or_doc_token.text_range().start(),
                    None,
                    first_non_attr_or_doc_token
                        .parent_node()
                        .and_then(|it| get_insert_indenting(&it)),
                )
            },
        )
}

/// Returns the insertion offset and affixes (i.e whitespace and delimiters e.g `(`, `,` and `)`) for an ink! attribute argument .
///
/// **NOTE**: For attributes that have values (e.g `selector = 1`), the equal symbol (`=`)
/// and the value are considered part of the attribute arguments (not suffixes),
/// so they're not handled by this function. See [`ink_arg_insertion_text`] doc instead.
pub fn ink_arg_insertion_offset_and_affixes(
    arg_kind: InkArgKind,
    ink_attr: &InkAttribute,
) -> Option<(TextSize, &str, &str)> {
    // Determines if its a "primary" attribute argument
    // as those get inserted at the beginning of the argument list while everything else gets inserted at the end.
    let is_primary_argument = matches!(
        arg_kind,
        InkArgKind::Constructor
            | InkArgKind::Event
            | InkArgKind::Extension
            | InkArgKind::Impl
            | InkArgKind::Message
            | InkArgKind::Storage
    );

    // Only computes insertion context for closed attributes because
    // unclosed attributes are too tricky for useful contextual edits.
    ink_attr.ast().r_brack_token().map(|r_bracket| {
        ink_attr.ast().token_tree().as_ref().map_or(
            (r_bracket.text_range().start(), "(", ")"),
            |token_tree| {
                (
                    // Computes the insertion offset.
                    if is_primary_argument {
                        // "Primary" attribute argument get inserted at the beginning of the argument list.
                        token_tree
                            .l_paren_token()
                            // Inserts just after left parenthesis if it exists, otherwise defaults to the end of the attribute.
                            .map_or(token_tree.syntax().text_range().end(), |r_paren| {
                                r_paren.text_range().end()
                            })
                    } else {
                        // Other attribute arguments get inserted at the end of the argument list.
                        token_tree
                            .r_paren_token()
                            // Inserts just before right parenthesis if it exists, otherwise defaults to the end of the attribute.
                            .map_or(token_tree.syntax().text_range().end(), |r_paren| {
                                r_paren.text_range().start()
                            })
                    },
                    // Determines the prefix to insert before the ink! attribute argument text.
                    match token_tree.l_paren_token() {
                        Some(_) => {
                            if is_primary_argument {
                                // No prefix for "primary" attribute arguments that already have a left parenthesis before them.
                                ""
                            } else {
                                // Determines prefix for "non-primary" attribute arguments that already have a left parenthesis before them.
                                token_tree
                                    .r_paren_token()
                                    .and_then(|r_paren| {
                                        r_paren.prev_token().map(|penultimate_token| {
                                            match penultimate_token.kind() {
                                                SyntaxKind::COMMA | SyntaxKind::L_PAREN => "",
                                                // Adds a comma if the token before the right parenthesis is
                                                // neither a comma nor the left parenthesis.
                                                _ => ", ",
                                            }
                                        })
                                    })
                                    .unwrap_or(
                                        match ink_analyzer_ir::last_child_token(token_tree.syntax())
                                        {
                                            Some(last_token) => match last_token.kind() {
                                                SyntaxKind::COMMA
                                                | SyntaxKind::L_PAREN
                                                | SyntaxKind::R_PAREN => "",
                                                // Adds a comma if there is no right parenthesis and the last token is
                                                // neither a comma nor the left parenthesis
                                                // (the right parenthesis in the pattern above will likely never match anything,
                                                // but parsers are weird :-) so we leave it for robustness? and clarity).
                                                _ => ", ",
                                            },
                                            None => "",
                                        },
                                    )
                            }
                        }
                        // Adds a left parenthesis if none already exists.
                        None => "(",
                    },
                    // Determines the suffix to insert after the ink! attribute argument text.
                    match token_tree.r_paren_token() {
                        Some(_) => {
                            if is_primary_argument {
                                // Determines suffix for "primary" attribute arguments that already have a right parenthesis after them.
                                token_tree
                                    .l_paren_token()
                                    .and_then(|l_paren| {
                                        l_paren.next_token().map(|first_token| {
                                            match first_token.kind() {
                                                SyntaxKind::COMMA | SyntaxKind::R_PAREN => "",
                                                // Adds a comma if the token after the left parenthesis is
                                                // neither a comma nor the right parenthesis.
                                                _ => ", ",
                                            }
                                        })
                                    })
                                    .unwrap_or(
                                        match ink_analyzer_ir::first_child_token(
                                            token_tree.syntax(),
                                        ) {
                                            Some(first_token) => match first_token.kind() {
                                                SyntaxKind::COMMA
                                                | SyntaxKind::L_PAREN
                                                | SyntaxKind::R_PAREN => "",
                                                // Adds a comma if there is no left parenthesis and the first token is
                                                // neither a comma nor the right parenthesis
                                                // (the left parenthesis in the pattern above will likely never match anything,
                                                // but parsers are weird :-) so we leave it for robustness? and clarity).
                                                _ => ", ",
                                            },
                                            None => "",
                                        },
                                    )
                            } else {
                                // No suffix for "non-primary" attribute arguments that already have a right parenthesis after them.
                                ""
                            }
                        }
                        // Adds a right parenthesis if none already exists.
                        None => ")",
                    },
                )
            },
        )
    })
}

/// Returns the indenting (preceding whitespace) of the syntax node.
pub fn item_indenting(node: &SyntaxNode) -> Option<String> {
    node.prev_sibling_or_token().and_then(|prev_elem| {
        (prev_elem.kind() == SyntaxKind::WHITESPACE).then_some(
            prev_elem
                .to_string()
                .chars()
                .rev()
                .take_while(|char| *char != '\n')
                .collect::<String>(),
        )
    })
}

/// Returns the deepest syntax element that fully covers text range (if any).
pub fn focused_element<T: FromSyntax>(item: &T, range: TextRange) -> Option<SyntaxElement> {
    if range.is_empty() {
        // Uses item at offset utility if the range start and end are equal.
        item.item_at_offset(range.start())
            .focused_token()
            .cloned()
            .map(SyntaxElement::Token)
    } else {
        item.syntax()
            .text_range()
            // Ensure the text range is in the bounds of the source code.
            .contains_range(range)
            .then(|| {
                // Retrieves deepest element that fully covers the text range.
                item.syntax().covering_element(range)
            })
    }
}

/// Returns the covering attribute for the text range (if any).
pub fn covering_attribute<T: FromSyntax>(item: &T, range: TextRange) -> Option<ast::Attr> {
    if range.is_empty() {
        // Uses item at offset utility if the range start and end are equal.
        // This way we keep some of the guarantees about parent AST items for unclosed attributes that
        // the item at offset utility enforces.
        item.item_at_offset(range.start()).parent_attr()
    } else {
        // Retrieves deepest element that fully covers the text range.
        focused_element(item, range).and_then(|covering_element| {
            if ast::Attr::can_cast(covering_element.kind()) {
                // Casts covering element to `ast::Attr` node if it's an attribute.
                covering_element.into_node().and_then(ast::Attr::cast)
            } else {
                // Finds the parent attribute (if any) of the covering element.
                ink_analyzer_ir::closest_ancestor_ast_type::<SyntaxElement, ast::Attr>(
                    &covering_element,
                )
            }
        })
    }
}

/// Returns the covering ink! attribute for the text range (if any).
pub fn covering_ink_attribute<T: FromSyntax>(item: &T, range: TextRange) -> Option<InkAttribute> {
    covering_attribute(item, range).and_then(InkAttribute::cast)
}

/// Returns the parent AST item for the text range (if any).
pub fn parent_ast_item<T: FromSyntax>(item: &T, range: TextRange) -> Option<ast::Item> {
    if range.is_empty() {
        // Uses item at offset utility if the range start and end are equal.
        // This way we keep some of the guarantees about parent AST items for unclosed attributes that
        // the item at offset utility enforces.
        item.item_at_offset(range.start()).parent_ast_item()
    } else {
        // Retrieves deepest element that fully covers the text range.
        focused_element(item, range).and_then(|covering_element| {
            if ast::Item::can_cast(covering_element.kind()) {
                // Casts covering element to `ast::Item` node if it's an AST item.
                covering_element.into_node().and_then(ast::Item::cast)
            } else {
                // Finds the parent AST item (if any) of the covering element.
                ink_analyzer_ir::closest_ancestor_ast_type::<SyntaxElement, ast::Item>(
                    &covering_element,
                )
            }
        })
    }
}
