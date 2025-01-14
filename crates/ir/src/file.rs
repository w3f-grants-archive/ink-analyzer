//! ink! source file IR.

use ra_ap_syntax::SourceFile;

use crate::{ChainExtension, Contract, InkE2ETest, InkTest, StorageItem, TraitDefinition};

/// An ink! file.
#[ink_analyzer_macro::entity]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InkFile {
    // ASTNode type.
    ast: SourceFile,
    // ink! contracts.
    contracts: Vec<Contract>,
    // ink! trait definitions.
    #[initializer(peek_macro = Contract)]
    trait_definitions: Vec<TraitDefinition>,
    // ink! chain extensions.
    #[initializer(peek_macro = Contract)]
    chain_extensions: Vec<ChainExtension>,
    // ink! storage items.
    #[initializer(peek_macro = Contract)]
    storage_items: Vec<StorageItem>,
    // ink! tests.
    tests: Vec<InkTest>,
    // ink! e2e tests.
    e2e_tests: Vec<InkE2ETest>,
}

impl InkFile {
    /// Parses ink! file from source code.
    pub fn parse(code: &str) -> Self {
        <Self as From<SourceFile>>::from(SourceFile::parse(code).tree())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::quote_as_str;

    #[test]
    fn parse_works() {
        let file = InkFile::parse(quote_as_str! {
            #[ink::contract]
            mod my_contract {
            }

            #[ink::trait_definition]
            pub trait MyTrait {
            }

            #[ink::chain_extension]
            pub trait MyChainExtension {
            }

            #[ink::storage_item]
            struct MyStorageItem {
            }

            #[ink::storage_item]
            struct MyStorageItem2 {
            }

            #[cfg(test)]
            mod tests {
                #[ink::test]
                fn it_works {
                }

                #[ink::test]
                fn it_works2 {
                }
            }
        });

        // 1 contract.
        assert_eq!(file.contracts().len(), 1);

        // 1 trait definition.
        assert_eq!(file.trait_definitions().len(), 1);

        // 1 chain extension.
        assert_eq!(file.chain_extensions().len(), 1);

        // 2 storage items.
        assert_eq!(file.storage_items().len(), 2);

        // 2 tests.
        assert_eq!(file.tests().len(), 2);
    }
}
