//! Coverage and instruction inspection for equivalence sampling.

use std::collections::BTreeSet;

use crate::eval::{CompiledExpr, Instr};
use crate::typecheck::Type;

use super::DomainVar;

pub(super) fn loads_of(expr: &CompiledExpr) -> Vec<u32> {
    let mut ids = BTreeSet::new();
    for instr in expr.code() {
        if let Instr::Load(id) = instr {
            ids.insert(*id);
        }
    }
    ids.into_iter().collect()
}

pub(super) fn missing_domain(lhs: &[u32], rhs: &[u32], domain: &[DomainVar]) -> Vec<u32> {
    let known: BTreeSet<u32> = domain.iter().map(|var| var.id).collect();
    lhs.iter()
        .chain(rhs)
        .copied()
        .filter(|id| !known.contains(id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn uses(expr: &CompiledExpr, predicate: impl Fn(&Instr) -> bool) -> bool {
    expr.code().iter().any(predicate)
}

pub(super) fn used_lookups(lhs: &CompiledExpr, rhs: &CompiledExpr) -> BTreeSet<u32> {
    let mut ids = BTreeSet::new();
    for expr in [lhs, rhs] {
        for instr in expr.code() {
            if let Instr::Lookup(table) = instr {
                ids.insert(*table);
            }
        }
    }
    ids
}

pub(super) fn describe_dim(ty: &Type) -> String {
    ty.dim()
        .map_or_else(|| "non-numeric".to_owned(), ToString::to_string)
}
