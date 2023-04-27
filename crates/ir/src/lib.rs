//! [ink!](https://use.ink/) intermediate representations (IRs) and abstractions
//! for [ink! analyzer](https://docs.rs/ink-analyzer/latest/ink_analyzer/).
//!
//! # Example
//! Generate an IR of ink! smart contract code.
//!
//! ```
//! use ink_analyzer_ir::{InkFile, quote_as_str};
//!
//! fn generate_ir() {
//!         let file = InkFile::parse(quote_as_str! {
//!             #[ink::contract]
//!             mod my_contract {
//!
//!                 #[ink(storage)]
//!                 pub struct MyContract {
//!                     value: bool,
//!                 }
//!
//!                 #[ink(event)]
//!                 pub struct MyEvent {
//!                     #[ink(topic)]
//!                     value: bool,
//!                 }
//!
//!                 // --snip--
//!             }
//!         });
//!         dbg!(&file);
//!
//!         let contracts = file.contracts();
//!         dbg!(&contracts);
//!
//!         if let Some(contract) = contracts.first() {
//!             let events = contract.events();
//!             dbg!(&events);
//!         }
//!     }
//! ```

pub use self::{
    attrs::{meta, InkArg, InkArgKind, InkAttrData, InkAttribute, InkAttributeKind, InkMacroKind},
    chain_extension::ChainExtension,
    constructor::Constructor,
    contract::Contract,
    event::Event,
    extension::Extension,
    file::InkFile,
    ink_impl::InkImpl,
    ink_test::InkTest,
    message::Message,
    storage::Storage,
    storage_item::StorageItem,
    topic::Topic,
    trait_definition::TraitDefinition,
    traits::{
        AsInkFn, AsInkImplItem, AsInkStruct, AsInkTrait, FromAST, FromInkAttribute, FromSyntax,
        IRItem,
    },
    utils::{
        ink_ancestors, ink_attrs, ink_attrs_ancestors, ink_attrs_closest_ancestors,
        ink_attrs_closest_descendants, ink_attrs_descendants, ink_attrs_in_scope,
        ink_callable_closest_descendants, ink_closest_ancestors, ink_closest_descendants,
        ink_descendants, ink_impl_closest_descendants, ink_parent, parent_ast_item,
    },
};

// Re-export ra_ap_syntax as syntax.
pub use ra_ap_syntax as syntax;

/// Re-export ra_ap_syntax::ast as ast.
pub use ra_ap_syntax::ast;

mod attrs;
mod chain_extension;
mod constructor;
mod contract;
mod event;
mod extension;
mod file;
mod ink_impl;
mod ink_test;
mod message;
mod storage;
mod storage_item;
mod topic;
mod trait_definition;

mod traits;
#[macro_use]
mod utils;
