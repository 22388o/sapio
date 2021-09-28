// Copyright Judica, Inc 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The different types of functionality a contract can define.
use std::sync::Arc;
use super::CompilationError;
use super::Context;
use super::TxTmplIt;
use sapio_base::Clause;
use schemars::schema::RootSchema;
use std::collections::LinkedList;
/// A Guard is a function which generates some condition that must be met to unlock a script.
/// If bool = true, the computation of the guard is cached, which is useful if e.g. Guard
/// must contact a remote server or it should be the same across calls *for a given contract
/// instance*.
pub enum Guard<ContractSelf> {
    /// Cache Variant should only be called one time per contract and the result saved
    Cache(fn(&ContractSelf, &Context) -> Clause),
    /// Fresh Variant may be called repeatedly
    Fresh(fn(&ContractSelf, &Context) -> Clause),
}

/// A List of Guards, for convenience
pub type GuardList<'a, T> = &'a [fn() -> Option<Guard<T>>];

/// Conditional Compilation function has specified that compilation of this
/// function should be required or not.
pub enum ConditionalCompileType {
    /// May proceed without calling this function at all
    Skippable,
    /// If no errors are returned, and no txtmpls are returned,
    /// it is not an error and the branch is pruned.
    Nullable,
    /// The default condition if no ConditionallyCompileIf function is set, the
    /// branch is present and it is required.
    Required,
    /// This branch must never be used
    Never,
    /// No Constraint, nothing is changed by this rule
    NoConstraint,
    /// The branch should always trigger an error, with some reasons
    Fail(LinkedList<String>),
}

impl ConditionalCompileType {
    /// Merge two `ConditionalCompileTypes` into one conditions.
    /// Precedence:
    ///     Fail > non-Fail ==> Fail
    ///     forall X. X > NoConstraint ==> X
    ///     Required > {Skippable, Nullable} ==> Required
    ///     Skippable > Nullable ==> Skippable
    ///     Never >< Required ==> Fail
    ///     Never > {Skippable, Nullable}  ==> Never
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (ConditionalCompileType::NoConstraint, x) => x,
            (x, ConditionalCompileType::NoConstraint) => x,
            // Merge error messages
            (ConditionalCompileType::Fail(mut v), ConditionalCompileType::Fail(mut v2)) => {
                ConditionalCompileType::Fail({
                    v.append(&mut v2);
                    v
                })
            }
            // Fail ignored and overrides other conditions.
            (ConditionalCompileType::Fail(v), _) | (_, ConditionalCompileType::Fail(v)) => {
                ConditionalCompileType::Fail(v)
            }
            // Never and Required Conflict
            (ConditionalCompileType::Required, ConditionalCompileType::Never)
            | (ConditionalCompileType::Never, ConditionalCompileType::Required) => {
                let mut l = LinkedList::new();
                l.push_front(String::from("Never and Required incompatible"));
                ConditionalCompileType::Fail(l)
            }
            // Never stays Never
            (ConditionalCompileType::Never, ConditionalCompileType::Skippable)
            | (ConditionalCompileType::Skippable, ConditionalCompileType::Never)
            | (ConditionalCompileType::Never, ConditionalCompileType::Nullable)
            | (ConditionalCompileType::Nullable, ConditionalCompileType::Never)
            | (ConditionalCompileType::Never, ConditionalCompileType::Never) => {
                ConditionalCompileType::Never
            }
            // Required stays Required
            (ConditionalCompileType::Required, ConditionalCompileType::Skippable)
            | (ConditionalCompileType::Skippable, ConditionalCompileType::Required)
            | (ConditionalCompileType::Required, ConditionalCompileType::Nullable)
            | (ConditionalCompileType::Nullable, ConditionalCompileType::Required)
            | (ConditionalCompileType::Required, ConditionalCompileType::Required) => {
                ConditionalCompileType::Required
            }
            (ConditionalCompileType::Skippable, ConditionalCompileType::Skippable)
            | (ConditionalCompileType::Skippable, ConditionalCompileType::Nullable)
            | (ConditionalCompileType::Nullable, ConditionalCompileType::Skippable) => {
                ConditionalCompileType::Skippable
            }
            (ConditionalCompileType::Nullable, ConditionalCompileType::Nullable) => {
                ConditionalCompileType::Nullable
            }
        }
    }
}

/// A `ConditionallyCompileIf` is a function wrapper which generates some
/// condition that must be met to disable a branch.
///
/// We use a separate function so that static analysis tools may operate without
/// running the actual `ThenFunc`.
pub enum ConditionallyCompileIf<ContractSelf> {
    /// Fresh Variant may be called repeatedly
    Fresh(fn(&ContractSelf, &Context) -> ConditionalCompileType),
}

/// A List of ConditionallyCompileIfs, for convenience
pub type ConditionallyCompileIfList<'a, T> = &'a [fn() -> Option<ConditionallyCompileIf<T>>];

/// A ThenFunc takes a list of Guards and a TxTmplIt generator.  Each TxTmpl returned from the
/// ThenFunc is Covenant Permitted only if the AND of all guards is satisfied.
pub struct ThenFunc<'a, ContractSelf: 'a> {
    /// Guards returns Clauses -- if any -- before the internal func's returned
    /// TxTmpls should execute on-chain
    pub guard: GuardList<'a, ContractSelf>,
    /// conditional_compile_if returns ConditionallyCompileType to determine if a function
    /// should be included.
    pub conditional_compile_if: ConditionallyCompileIfList<'a, ContractSelf>,
    /// func returns an iterator of possible transactions
    /// Implementors should aim to return as few `TxTmpl`s as possible for enhanced
    /// semantics, preferring to split across multiple `ThenFunc`'s
    pub func: fn(&ContractSelf, &Context) -> TxTmplIt,
}

/// A function which by default finishes, but may receive some context object which can induce the
/// generation of additional transactions (as a suggestion)
pub struct FinishOrFunc<'a, ContractSelf: 'a, StatefulArguments, SpecificArgs> {
    /// StatefulArgs is needed to capture a general API for all calls, but SpecificArgs is required
    /// for a given function.
    pub coerce_args: fn(StatefulArguments) -> Result<SpecificArgs, CompilationError>,
    /// Guards returns Clauses -- if any -- before the coins should be unlocked
    pub guard: GuardList<'a, ContractSelf>,
    /// conditional_compile_if returns ConditionallyCompileType to determine if a function
    /// should be included.
    pub conditional_compile_if: ConditionallyCompileIfList<'a, ContractSelf>,
    /// func returns an iterator of possible transactions
    /// Implementors should aim to return as few `TxTmpl`s as possible for enhanced
    /// semantics, preferring to split across multiple `FinishOrFunc`'s.
    /// These `TxTmpl`s are non-binding, merely suggested.
    pub func: fn(&ContractSelf, &Context, SpecificArgs) -> TxTmplIt,
    /// to be filled in if SpecificArgs has a schema, which it might not.
    /// because negative trait bounds do not exists, that is up to the
    /// implementation to decide if the trait exists.
    pub schema: Option<Arc<RootSchema>>,
    /// name derived from Function Name.
    pub name: String,
}

/// This trait hides the generic parameter `SpecificArgs` in FinishOrFunc
/// through a trait object interface which enables FinishOrFuncs to have a
/// custom type per fucntion, so long as there is a way to convert from
/// StatefulArguments to SpecificArgs via coerce_args. By default, this is
/// presently done through `std::convert::TryInto::try_into`.
pub trait CallableAsFoF<ContractSelf, StatefulArguments> {
    /// Calls the internal function, should convert `StatefulArguments` to `SpecificArgs`.
    fn call(&self, cself: &ContractSelf, ctx: &Context, o: StatefulArguments) -> TxTmplIt;
    /// Getter Method for internal field
    fn get_conditional_compile_if(&self) -> ConditionallyCompileIfList<'_, ContractSelf>;
    /// Getter Method for internal field
    fn get_guard(&self) -> GuardList<'_, ContractSelf>;
    /// Get the name for this function
    fn get_name(&self) -> &str;
    /// Get the RootSchema for calling this with an update
    fn get_schema(&self) -> &Option<Arc<RootSchema>>;
}
impl<ContractSelf, StatefulArguments, SpecificArgs> CallableAsFoF<ContractSelf, StatefulArguments>
    for FinishOrFunc<'_, ContractSelf, StatefulArguments, SpecificArgs>
{
    fn call(&self, cself: &ContractSelf, ctx: &Context, o: StatefulArguments) -> TxTmplIt {
        let args = (self.coerce_args)(o)?;
        (self.func)(cself, ctx, args)
    }
    fn get_conditional_compile_if(&self) -> ConditionallyCompileIfList<'_, ContractSelf> {
        self.conditional_compile_if
    }
    fn get_guard(&self) -> GuardList<'_, ContractSelf> {
        self.guard
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_schema(&self) -> &Option<Arc<RootSchema>> {
        &self.schema
    }
}
