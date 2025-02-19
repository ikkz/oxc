use oxc_ast::{visit::walk::walk_block_statement, AstKind, Visit};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_semantic::ScopeFlags;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use crate::{ast_util::is_function_kind, context::LintContext, rule::Rule};

fn max_statements_diagnostic(name: &str, count: usize, max: usize, span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!(
        "{name} has too many statements ({count}). Maximum allowed is {max}."
    ))
    .with_help("Consider splitting it into smaller functions.")
    .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct MaxStatements {
    max: usize,
    ignore_top_level_functions: bool,
}

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Enforce a maximum number of statements allowed in function blocks
    ///
    /// This rule allows you to specify the maximum number of statements allowed in a
    /// function.
    ///
    /// ```js
    /// function foo() {
    ///   const bar = 1; // one statement
    ///   const baz = 2; // two statements
    ///   const qux = 3; // three statements
    /// }
    /// ```
    ///
    /// ### Why is this bad?
    ///
    /// Some people consider large functions a code smell. Large functions tend to do a
    /// lot of things and can make it hard following what’s going on. Many coding style
    /// guides dictate a limit of the number of lines that a function can comprise of.
    /// This rule can help enforce that style.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule with the default `{ "max": 10 }`
    /// option:
    /// ```js
    /// function foo() {
    ///   const foo1 = 1;
    ///   const foo2 = 2;
    ///   const foo3 = 3;
    ///   const foo4 = 4;
    ///   const foo5 = 5;
    ///   const foo6 = 6;
    ///   const foo7 = 7;
    ///   const foo8 = 8;
    ///   const foo9 = 9;
    ///   const foo10 = 10;
    ///
    ///   const foo11 = 11; // Too many.
    /// }
    ///
    /// const bar = () => {
    ///   const foo1 = 1;
    ///   const foo2 = 2;
    ///   const foo3 = 3;
    ///   const foo4 = 4;
    ///   const foo5 = 5;
    ///   const foo6 = 6;
    ///   const foo7 = 7;
    ///   const foo8 = 8;
    ///   const foo9 = 9;
    ///   const foo10 = 10;
    ///
    ///   const foo11 = 11; // Too many.
    /// };
    /// ```
    ///
    /// Examples of **correct** code for this rule with the default `{ "max": 10 }`
    /// option:
    /// ```js
    /// function foo() {
    ///   const foo1 = 1;
    ///   const foo2 = 2;
    ///   const foo3 = 3;
    ///   const foo4 = 4;
    ///   const foo5 = 5;
    ///   const foo6 = 6;
    ///   const foo7 = 7;
    ///   const foo8 = 8;
    ///   const foo9 = 9;
    ///   return function () { // 10
    ///
    ///     // The number of statements in the inner function does not count toward the
    ///     // statement maximum.
    ///
    ///     let bar;
    ///     let baz;
    ///     return 42;
    ///   };
    /// }
    ///
    /// const bar = () => {
    ///   const foo1 = 1;
    ///   const foo2 = 2;
    ///   const foo3 = 3;
    ///   const foo4 = 4;
    ///   const foo5 = 5;
    ///   const foo6 = 6;
    ///   const foo7 = 7;
    ///   const foo8 = 8;
    ///   const foo9 = 9;
    ///   return function () { // 10
    ///
    ///     // The number of statements in the inner function does not count toward the
    ///     // statement maximum.
    ///
    ///     let bar;
    ///     let baz;
    ///     return 42;
    ///   };
    /// }
    /// ```
    ///
    /// Note that this rule does not apply to class static blocks, and that statements in
    /// class static blocks do not count as statements in the enclosing function.
    ///
    /// Examples of **correct** code for this rule with `{ "max": 2 }` option:
    /// ```js
    /// function foo() {
    ///     let one;
    ///     let two = class {
    ///         static {
    ///             let three;
    ///             let four;
    ///             let five;
    ///             if (six) {
    ///                 let seven;
    ///                 let eight;
    ///                 let nine;
    ///             }
    ///         }
    ///     };
    /// }
    /// ```
    ///
    /// Examples of additional **correct** code for this rule with the
    /// `{ "max": 10 }, { "ignoreTopLevelFunctions": true }` options:
    /// ```js
    /// function foo() {
    ///   const foo1 = 1;
    ///   const foo2 = 2;
    ///   const foo3 = 3;
    ///   const foo4 = 4;
    ///   const foo5 = 5;
    ///   const foo6 = 6;
    ///   const foo7 = 7;
    ///   const foo8 = 8;
    ///   const foo9 = 9;
    ///   const foo10 = 10;
    ///   const foo11 = 11;
    /// }
    /// ```
    /// ### Options
    ///
    /// #### max
    ///
    /// `{ type: number, default: 10 }`
    ///
    /// The `max` enforces a maximum number of statements allows in function blocks
    ///
    /// #### ignoreTopLevelFunctions
    ///
    /// `{ type: boolean, default: true }`
    ///
    /// Whether to ignores top-level functions
    ///
    /// Example:
    /// ```json
    /// "eslint/max-statements": ["error", 10]
    ///
    /// "eslint/max-statements": [
    ///   "error",
    ///   10,
    ///   {
    ///     "ignoreTopLevelFunctions": true
    ///   }
    /// ]
    ///
    /// ```
    MaxStatements,
    eslint,
    pedantic
);

impl Rule for MaxStatements {
    fn run_once(&self, ctx: &LintContext) {}

    fn from_configuration(value: serde_json::Value) -> Self {
        let max = value
            .get(0)
            .and_then(Value::as_number)
            .and_then(serde_json::Number::as_u64)
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(10);
        let ignore_top_level_functions = value
            .get(1)
            .and_then(|config| config.get("ignoreTopLevelFunctions"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        Self { max, ignore_top_level_functions }
    }
}

fn is_counter_unit(kind: &AstKind<'_>) -> bool {
    is_function_kind(kind) || matches!(kind, AstKind::StaticBlock(_))
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        ("function foo() { var bar = 1; function qux () { var noCount = 2; } return 3; }", Some(serde_json::json!([3]))),
        ("function foo() { var bar = 1; if (true) { for (;;) { var qux = null; } } else { quxx(); } return 3; }", Some(serde_json::json!([6]))),
        ("function foo() { var x = 5; function bar() { var y = 6; } bar(); z = 10; baz(); }", Some(serde_json::json!([5]))),
        ("function foo() { var a; var b; var c; var x; var y; var z; bar(); baz(); qux(); quxx(); }", None),
        ("(function() { var bar = 1; return function () { return 42; }; })()", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        ("function foo() { var bar = 1; var baz = 2; }", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        ("define(['foo', 'qux'], function(foo, qux) { var bar = 1; var baz = 2; })", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        ("var foo = { thing: function() { var bar = 1; var baz = 2; } }", Some(serde_json::json!([2]))),
        ("var foo = { thing() { var bar = 1; var baz = 2; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = { ['thing']() { var bar = 1; var baz = 2; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = { thing: () => { var bar = 1; var baz = 2; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = { thing: function() { var bar = 1; var baz = 2; } }", Some(serde_json::json!([{ "max": 2 }]))),
        ("class C { static { one; two; three; { four; five; six; } } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 },
        ("function foo() { class C { static { one; two; three; { four; five; six; } } } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 },
        ("class C { static { one; two; three; function foo() { 1; 2; } four; five; six; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 },
        ("class C { static { { one; two; three; function foo() { 1; 2; } four; five; six; } } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 },
        ("function top_level() { 1; /* 2 */ class C { static { one; two; three; { four; five; six; } } } 3;}", Some(serde_json::json!([2, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 },
        ("function top_level() { 1; 2; } class C { static { one; two; three; { four; five; six; } } }", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 },
        ("class C { static { one; two; three; { four; five; six; } } } function top_level() { 1; 2; } ", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 },
        ("function foo() { let one; let two = class { static { let three; let four; let five; if (six) { let seven; let eight; let nine; } } }; }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 }
    ];

    let fail = vec![
        ("function foo() { var bar = 1; var baz = 2; var qux = 3; }", Some(serde_json::json!([2]))),
        ("var foo = () => { var bar = 1; var baz = 2; var qux = 3; };", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = function() { var bar = 1; var baz = 2; var qux = 3; };", Some(serde_json::json!([2]))),
        ("function foo() { var bar = 1; if (true) { while (false) { var qux = null; } } return 3; }", Some(serde_json::json!([4]))),
        ("function foo() { var bar = 1; if (true) { for (;;) { var qux = null; } } return 3; }", Some(serde_json::json!([4]))),
        ("function foo() { var bar = 1; if (true) { for (;;) { var qux = null; } } else { quxx(); } return 3; }", Some(serde_json::json!([5]))),
        ("function foo() { var x = 5; function bar() { var y = 6; } bar(); z = 10; baz(); }", Some(serde_json::json!([3]))),
        ("function foo() { var x = 5; function bar() { var y = 6; } bar(); z = 10; baz(); }", Some(serde_json::json!([4]))),
        (";(function() { var bar = 1; return function () { var z; return 42; }; })()", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        (";(function() { var bar = 1; var baz = 2; })(); (function() { var bar = 1; var baz = 2; })()", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        ("define(['foo', 'qux'], function(foo, qux) { var bar = 1; var baz = 2; return function () { var z; return 42; }; })", Some(serde_json::json!([1, { "ignoreTopLevelFunctions": true }]))),
        ("function foo() { var a; var b; var c; var x; var y; var z; bar(); baz(); qux(); quxx(); foo(); }", None),
        ("var foo = { thing: function() { var bar = 1; var baz = 2; var baz2; } }", Some(serde_json::json!([2]))),
        ("var foo = { thing() { var bar = 1; var baz = 2; var baz2; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = { thing: () => { var bar = 1; var baz = 2; var baz2; } }", Some(serde_json::json!([2]))), // { "ecmaVersion": 6 },
        ("var foo = { thing: function() { var bar = 1; var baz = 2; var baz2; } }", Some(serde_json::json!([{ "max": 2 }]))),
        ("function foo() { 1; 2; 3; 4; 5; 6; 7; 8; 9; 10; 11; }", Some(serde_json::json!([{}]))),
        ("function foo() { 1; }", Some(serde_json::json!([{ "max": 0 }]))),
        ("function foo() { foo_1; /* foo_ 2 */ class C { static { one; two; three; four; { five; six; seven; eight; } } } foo_3 }", Some(serde_json::json!([2]))), // { "ecmaVersion": 2022 },
        ("class C { static { one; two; three; four; function not_top_level() { 1; 2; 3; } five; six; seven; eight; } }", Some(serde_json::json!([2, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 },
        ("class C { static { { one; two; three; four; function not_top_level() { 1; 2; 3; } five; six; seven; eight; } } }", Some(serde_json::json!([2, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 },
        ("class C { static { { one; two; three; four; } function not_top_level() { 1; 2; 3; } { five; six; seven; eight; } } }", Some(serde_json::json!([2, { "ignoreTopLevelFunctions": true }]))), // { "ecmaVersion": 2022 }
    ];

    Tester::new(MaxStatements::NAME, MaxStatements::PLUGIN, pass, fail).test_and_snapshot();
}
