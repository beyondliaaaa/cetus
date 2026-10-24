use super::*;
use crate::eval::compile;
use crate::typecheck::{NameResolver, Resolved, TableSignature, Type as ExprType, parse_and_check};
use cetus_units::{Dim, UnitRegistry};

struct Resolver;

impl NameResolver for Resolver {
    fn resolve(&self, name: &str) -> Option<Resolved> {
        let id = match name {
            "a" => 0,
            "b" => 1,
            "c" => 2,
            _ => return None,
        };
        Some(Resolved::Variable {
            id,
            ty: ExprType::scalar(Dim::dimensionless()),
        })
    }
}

struct TableResolver;

impl NameResolver for TableResolver {
    fn resolve(&self, name: &str) -> Option<Resolved> {
        (name == "a").then(|| Resolved::Variable {
            id: 0,
            ty: ExprType::scalar(Dim::dimensionless()),
        })
    }

    fn resolve_table(&self, name: &str) -> Option<TableSignature> {
        (name == "t").then(|| TableSignature {
            name: "t".to_owned(),
            id: 0,
            input: Dim::dimensionless(),
            output: Dim::dimensionless(),
        })
    }
}

struct Doubling;

impl TableLookup for Doubling {
    fn lookup(&self, table: u32, x: f64) -> Option<f64> {
        (table == 0).then_some(2.0 * x)
    }
}

fn compile_expr(source: &str) -> CompiledExpr {
    let registry = UnitRegistry::new();
    compile(&parse_and_check(source, &Resolver, &registry).expect("typecheck"))
}

fn compile_table_expr(source: &str) -> CompiledExpr {
    let registry = UnitRegistry::new();
    compile(&parse_and_check(source, &TableResolver, &registry).expect("typecheck"))
}

fn domain() -> Vec<DomainVar> {
    vec![
        DomainVar {
            id: 0,
            name: "a".to_owned(),
            min: 0.1,
            max: 10.0,
        },
        DomainVar {
            id: 1,
            name: "b".to_owned(),
            min: 0.1,
            max: 10.0,
        },
        DomainVar {
            id: 2,
            name: "c".to_owned(),
            min: 0.1,
            max: 10.0,
        },
    ]
}

fn tiny_domain() -> Vec<DomainVar> {
    vec![DomainVar {
        id: 0,
        name: "a".to_owned(),
        min: 1e-12,
        max: 2e-12,
    }]
}

#[test]
fn detects_algebraic_equivalence() {
    let left = compile_expr("a*b");
    let right = compile_expr("b*a");
    assert_eq!(
        equivalent(&left, &right, &domain(), &EquivOptions::default()),
        EquivOutcome::Equivalent
    );
}

#[test]
fn returns_a_counterexample_for_a_wrong_formula() {
    let left = compile_expr("a*b");
    let right = compile_expr("a*b*b");
    match equivalent(&left, &right, &domain(), &EquivOptions::default()) {
        EquivOutcome::Different {
            example, rel_error, ..
        } => {
            assert_eq!(example.len(), 3);
            assert!(rel_error > 0.0);
        }
        other => panic!("expected Different, got {other:?}"),
    }
}

#[test]
fn dimension_mismatch_is_reported() {
    let left = compile_expr("1[m]");
    let right = compile_expr("1[s]");
    assert!(matches!(
        equivalent(&left, &right, &[], &EquivOptions::default()),
        EquivOutcome::TypeMismatch { .. }
    ));
}

#[test]
fn time_dependent_expressions_require_a_time_range() {
    let left = compile_expr("sin(time / 1[s])");
    let right = compile_expr("sin(2 * time / 1[s])");
    match equivalent(&left, &right, &[], &EquivOptions::default()) {
        EquivOutcome::Undefined { message } => assert!(message.contains("time"), "{message}"),
        other => panic!("expected Undefined, got {other:?}"),
    }
}

#[test]
fn time_is_sampled_instead_of_fixed_at_zero() {
    let left = compile_expr("sin(time / 1[s])");
    let right = compile_expr("sin(2 * time / 1[s])");
    let options = EquivOptions {
        time: Some((0.0, 10.0)),
        ..EquivOptions::default()
    };
    match equivalent(&left, &right, &[], &options) {
        EquivOutcome::Different { example, .. } => {
            assert!(
                example.iter().any(|(name, _)| name == "time"),
                "{example:?}"
            );
        }
        other => panic!("expected Different, got {other:?}"),
    }
}

#[test]
fn tiny_quantities_are_compared_relatively() {
    let left = compile_expr("1e-9");
    let right = compile_expr("1.1e-9");
    assert!(matches!(
        equivalent(&left, &right, &[], &EquivOptions::default()),
        EquivOutcome::Different { .. }
    ));
}

#[test]
fn declared_scale_is_the_absolute_tolerance_floor() {
    let left = compile_expr("a");
    let right = compile_expr("a + 5e-19");
    let with_scale = EquivOptions {
        scale: Some(1e-9),
        ..EquivOptions::default()
    };
    assert_eq!(
        equivalent(&left, &right, &tiny_domain(), &with_scale),
        EquivOutcome::Equivalent
    );
    assert!(matches!(
        equivalent(&left, &right, &tiny_domain(), &EquivOptions::default()),
        EquivOutcome::Different { .. }
    ));
}

#[test]
fn missing_domain_coverage_is_reported() {
    let left = compile_expr("a*b");
    let right = compile_expr("b*a");
    let only_b = vec![DomainVar {
        id: 1,
        name: "b".to_owned(),
        min: 0.1,
        max: 10.0,
    }];
    match equivalent(&left, &right, &only_b, &EquivOptions::default()) {
        EquivOutcome::IncompleteDomain { missing } => assert_eq!(missing, vec![0]),
        other => panic!("expected IncompleteDomain, got {other:?}"),
    }
}

#[test]
fn lookup_without_tables_is_rejected() {
    let left = compile_table_expr("lookup(t, a)");
    let right = compile_expr("2 * a");
    match equivalent(&left, &right, &domain(), &EquivOptions::default()) {
        EquivOutcome::Undefined { message } => {
            assert!(message.contains("lookup table"), "{message}");
        }
        other => panic!("expected Undefined, got {other:?}"),
    }
}

#[test]
fn lookup_tables_are_used_when_provided() {
    let left = compile_table_expr("lookup(t, a)");
    let right = compile_expr("2 * a");
    assert_eq!(
        equivalent_with_tables(
            &left,
            &right,
            &domain(),
            &EquivOptions::default(),
            &Doubling
        ),
        EquivOutcome::Equivalent
    );
    let wrong = compile_expr("3 * a");
    assert!(matches!(
        equivalent_with_tables(
            &left,
            &wrong,
            &domain(),
            &EquivOptions::default(),
            &Doubling
        ),
        EquivOutcome::Different { .. }
    ));
}

#[test]
fn stateful_builtins_are_not_comparable() {
    let left = compile_expr("smooth(a, 1[s])");
    let right = compile_expr("a");
    match equivalent(&left, &right, &domain(), &EquivOptions::default()) {
        EquivOutcome::Undefined { message } => assert!(message.contains("stateful"), "{message}"),
        other => panic!("expected Undefined, got {other:?}"),
    }
}

#[test]
fn dt_dependent_expressions_require_a_dt() {
    let left = compile_expr("dt");
    let right = compile_expr("dt");
    match equivalent(&left, &right, &[], &EquivOptions::default()) {
        EquivOutcome::Undefined { message } => assert!(message.contains("dt"), "{message}"),
        other => panic!("expected Undefined, got {other:?}"),
    }
    let options = EquivOptions {
        dt: Some(0.1),
        ..EquivOptions::default()
    };
    assert_eq!(
        equivalent(&left, &right, &[], &options),
        EquivOutcome::Equivalent
    );
}
