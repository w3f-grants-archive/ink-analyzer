//! ink! file level diagnostics.

use ink_analyzer_ir::{InkAttributeKind, InkFile};

use super::{
    chain_extension, contract, ink_e2e_test, ink_test, storage_item, trait_definition, utils,
};
use crate::{Diagnostic, Severity};

/// Runs ink! file level diagnostics.
pub fn diagnostics(results: &mut Vec<Diagnostic>, file: &InkFile) {
    // Runs generic diagnostics `utils::run_generic_diagnostics` doc.
    utils::run_generic_diagnostics(results, file);

    // Ensures that at most one ink! contract, See `ensure_contract_quantity`.
    ensure_contract_quantity(results, file);

    // ink! contract diagnostics.
    for item in file.contracts() {
        contract::diagnostics(results, item);
    }

    // Runs ink! trait definition diagnostics, see `trait_definition::diagnostics` doc.
    for item in file.trait_definitions() {
        trait_definition::diagnostics(results, item);
    }

    // Runs ink! chain extension diagnostics, see `chain_extension::diagnostics` doc.
    for item in file.chain_extensions() {
        chain_extension::diagnostics(results, item);
    }

    // Runs ink! storage item diagnostics, see `storage_item::diagnostics` doc.
    for item in file.storage_items() {
        storage_item::diagnostics(results, item);
    }

    // Runs ink! test diagnostics, see `ink_test::diagnostics` doc.
    for item in file.tests() {
        ink_test::diagnostics(results, item);
    }

    // Runs ink! e2e test diagnostics, see `ink_e2e_test::diagnostics` doc.
    for item in file.e2e_tests() {
        ink_e2e_test::diagnostics(results, item);
    }

    // Ensures that only ink! attribute macro quasi-direct descendants (i.e ink! descendants without any ink! ancestors),
    // See `ensure_valid_quasi_direct_ink_descendants` doc.
    ensure_valid_quasi_direct_ink_descendants(results, file);
}

/// Ensures that there are not multiple ink! contract definitions.
///
/// Multiple ink! contract definitions in a single file generate conflicting metadata definitions.
///
/// Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/codegen/src/generator/metadata.rs#L51>.
fn ensure_contract_quantity(results: &mut Vec<Diagnostic>, file: &InkFile) {
    utils::ensure_at_most_one_item(
        results,
        file.contracts(),
        "Only one ink! contract per file is currently supported.",
        Severity::Error,
    );
}

/// Ensures that only ink! attribute macro quasi-direct descendants (i.e ink! descendants without any ink! ancestors).
///
/// Ref: <https://github.com/paritytech/ink/blob/v4.1.0/crates/ink/ir/src/ir/item/mod.rs#L98-L114>.
fn ensure_valid_quasi_direct_ink_descendants(results: &mut Vec<Diagnostic>, file: &InkFile) {
    utils::ensure_valid_quasi_direct_ink_descendants(results, file, |attr| {
        matches!(attr.kind(), InkAttributeKind::Macro(_))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::verify_actions;
    use ink_analyzer_ir::InkFile;
    use quote::format_ident;
    use test_utils::{quote_as_pretty_string, quote_as_str, TestResultAction, TestResultTextRange};

    #[test]
    fn one_contract_definition_works() {
        let file = InkFile::parse(quote_as_str! {
            #[ink::contract]
            mod my_contract {
            }
        });

        let mut results = Vec::new();
        ensure_contract_quantity(&mut results, &file);
        assert!(results.is_empty());
    }

    #[test]
    fn no_contract_definitions_works() {
        let file = InkFile::parse("");

        let mut results = Vec::new();
        ensure_contract_quantity(&mut results, &file);
        assert!(results.is_empty());
    }

    #[test]
    fn multiple_contract_definitions_fails() {
        // Tests snippets with btn 2 and 5 contract definitions.
        for idx in 2..=5 {
            // Creates code snippets with multiple contract definitions.
            let code = (1..=idx)
                .map(|i| {
                    let name = format_ident!("my_contract{i}");
                    (quote_as_pretty_string! {
                        #[ink::contract]
                        mod #name {
                        }
                    })
                    .to_string()
                })
                .collect::<String>();

            let file = InkFile::parse(&code);

            let mut results = Vec::new();
            ensure_contract_quantity(&mut results, &file);

            // There should be `idx-1` extraneous contract definitions.
            assert_eq!(results.len(), idx - 1);
            // All diagnostics should be errors.
            assert_eq!(
                results
                    .iter()
                    .filter(|item| item.severity == Severity::Error)
                    .count(),
                idx - 1
            );
            // Verifies quickfixes.
            if let Some(quickfixes) = &results[0].quickfixes {
                for fix in quickfixes {
                    let remove_item_or_attribute = fix.label.contains("Remove `#[ink::contract]`")
                        || fix.label.contains("Remove item");
                    assert!(remove_item_or_attribute);
                }
            }
        }
    }

    #[test]
    fn valid_quasi_direct_descendant_works() {
        let contract = InkFile::parse(quote_as_str! {
            #[ink::contract]
            mod my_contract {
            }

            #[ink::trait_definition]
            trait MyTrait {
            }

            #[ink::chain_extension]
            trait MyChainExtension {
            }

            #[ink::storage_item]
            struct MyStorageItem {
            }

            #[cfg(test)]
            mod tests {
                #[ink::test]
                fn it_works() {

                }
            }
        });

        let mut results = Vec::new();
        ensure_valid_quasi_direct_ink_descendants(&mut results, &contract);
        assert!(results.is_empty());
    }

    #[test]
    fn invalid_quasi_direct_descendant_fails() {
        let code = quote_as_pretty_string! {
            #[ink(storage)]
            struct MyContract {}

            #[ink(event)]
            struct MyEvent {
                #[ink(topic)]
                value: bool,
            }

            impl MyContract {
                #[ink(constructor)]
                pub fn my_constructor() -> Self {}

                #[ink(message)]
                pub fn my_message(&mut self) {}
            }
        };
        let contract = InkFile::parse(&code);

        let mut results = Vec::new();
        ensure_valid_quasi_direct_ink_descendants(&mut results, &contract);

        // There should be 4 errors (i.e `storage`, `event`, `constructor` and `message`,
        // `topic` is not a quasi-direct dependant because it has `event` as a parent).
        assert_eq!(results.len(), 4);
        // All diagnostics should be errors.
        assert_eq!(
            results
                .iter()
                .filter(|item| item.severity == Severity::Error)
                .count(),
            4
        );
        // Verifies quickfixes.
        let expected_quickfixes = vec![
            vec![
                TestResultAction {
                    label: "Remove `#[ink(storage)]`",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(storage)]"),
                        end_pat: Some("#[ink(storage)]"),
                    }],
                },
                TestResultAction {
                    label: "Remove item",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(storage)]"),
                        end_pat: Some("struct MyContract {}"),
                    }],
                },
            ],
            vec![
                TestResultAction {
                    label: "Remove `#[ink(event)]`",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(event)]"),
                        end_pat: Some("#[ink(event)]"),
                    }],
                },
                TestResultAction {
                    label: "Remove item",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(event)]"),
                        end_pat: Some("value: bool,\n}"),
                    }],
                },
            ],
            vec![
                TestResultAction {
                    label: "Remove `#[ink(constructor)]`",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(constructor)]"),
                        end_pat: Some("#[ink(constructor)]"),
                    }],
                },
                TestResultAction {
                    label: "Remove item",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(constructor)]"),
                        end_pat: Some("pub fn my_constructor() -> Self {}"),
                    }],
                },
            ],
            vec![
                TestResultAction {
                    label: "Remove `#[ink(message)]`",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(message)]"),
                        end_pat: Some("#[ink(message)]"),
                    }],
                },
                TestResultAction {
                    label: "Remove item",
                    edits: vec![TestResultTextRange {
                        text: "",
                        start_pat: Some("<-#[ink(message)]"),
                        end_pat: Some("pub fn my_message(&mut self) {}"),
                    }],
                },
            ],
        ];
        for (idx, item) in results.iter().enumerate() {
            let quickfixes = item.quickfixes.as_ref().unwrap();
            verify_actions(&code, quickfixes, &expected_quickfixes[idx]);
        }
    }
}
