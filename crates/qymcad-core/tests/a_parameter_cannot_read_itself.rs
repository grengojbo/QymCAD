//! A PARAMETER THAT READS ITSELF IS AN ERROR, NOT A NUMBER.
//!
//! Reported behaviour: `h = h * 2` with `h` at 10 settled on 5120. The parameters are evaluated to a fixed
//! point over up to eight passes, each name seeded with its previous value, so a formula that reads its own
//! name doubles on every pass and the eighth doubling is taken as the answer. A cycle through two names
//! (`a = b + 1`, `b = a`) drifts the same way, by one per pass.
//!
//! Such a formula has no value at all. It is reported the way an unparsable formula is: the parameter is
//! named among the errors and keeps the value it had.
use qymcad_core::errors::ExprError;
use qymcad_core::model::{Param, Project};

fn param(name: &str, expr: &str, value: f64) -> Param {
    Param { name: name.into(), expr: expr.into(), value }
}

fn project(parameters: Vec<Param>) -> Project {
    Project { parameters, ..Default::default() }
}

fn value(p: &Project, name: &str) -> f64 {
    p.parameters.iter().find(|q| q.name == name).map(|q| q.value).expect("the parameter exists")
}

/// A FORMULA THAT READS ITS OWN NAME keeps the value and is named among the errors.
#[test]
fn a_self_reference_is_reported_and_the_value_stays() {
    let mut p = project(vec![param("h", "h * 2", 10.0)]);
    let errs = p.eval_parameters();
    assert_eq!(value(&p, "h"), 10.0, "h = h * 2 must keep its value, not double on every pass");
    assert_eq!(errs, vec![("h".to_string(), ExprError::Cycle("h".into()))], "h = h * 2 must be reported as a cycle");
}

/// A CYCLE THROUGH TWO NAMES is the same fault one step longer: both are reported, both keep their values.
#[test]
fn a_cycle_through_two_parameters_is_reported_and_the_values_stay() {
    let mut p = project(vec![param("a", "b + 1", 1.0), param("b", "A", 2.0)]);
    let errs = p.eval_parameters();
    assert_eq!(value(&p, "a"), 1.0, "a = b + 1 in a cycle must keep its value");
    assert_eq!(value(&p, "b"), 2.0, "b = a in a cycle must keep its value");
    for n in ["a", "b"] {
        assert!(errs.contains(&(n.to_string(), ExprError::Cycle(n.into()))), "{n} sits in a cycle and must be reported, got {errs:?}");
    }
}

/// A CHAIN IS NOT A CYCLE: names that read each other one way only still evaluate, and a parameter that reads a
/// cycle from outside it takes the values the cycle kept.
#[test]
fn a_chain_still_evaluates_and_only_the_cycle_is_reported() {
    let mut p = project(vec![param("c", "b * 2", 0.0), param("b", "w + 1", 0.0), param("w", "40", 0.0), param("x", "x + 1", 5.0), param("y", "x * 10", 0.0)]);
    let errs = p.eval_parameters();
    assert_eq!(value(&p, "w"), 40.0);
    assert_eq!(value(&p, "b"), 41.0);
    assert_eq!(value(&p, "c"), 82.0);
    assert_eq!(value(&p, "x"), 5.0, "x = x + 1 keeps its value");
    assert_eq!(value(&p, "y"), 50.0, "y reads the value x kept");
    let named: Vec<&str> = errs.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(named, vec!["x"], "only the parameter in the cycle is reported");
}

/// THE FIELD OF THE PARAMETER WINDOW asks the same question before Enter: the formula typed for `h` is judged as
/// the formula of `h`, not as a free expression evaluated against the value `h` has now.
#[test]
fn a_formula_typed_for_a_parameter_is_judged_as_its_own() {
    let p = project(vec![param("h", "10", 10.0), param("a", "b + 1", 3.0), param("b", "2", 2.0)]);
    assert_eq!(p.eval_param_expr("h", "H * 2"), Err(ExprError::Cycle("h".into())));
    assert_eq!(p.eval_param_expr("b", "a"), Err(ExprError::Cycle("b".into())), "b = a closes the cycle through a = b + 1");
    assert_eq!(p.eval_param_expr("h", "a * 2"), Ok(6.0), "a formula that does not come back to h is a number");
    assert_eq!(p.eval_expr("h * 2"), Ok(20.0), "a free expression still reads h");
}
