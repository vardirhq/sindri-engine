//! Lists and ranges at runtime: written, walked, asked, and changed in place.

use crate::{EmptyHost, LIST_LIMIT, Runtime, RuntimeError, Value};

const SOURCE: &str = r#"script Bag {
    var held: List<f32> = [];
    var names = ["a", "b"];
    fn sum_to(n: f32) -> f32 {
        var total = 0.0;
        for i in 0.0..n { total += i; }
        return total;
    }
    fn skip_odd() -> f32 {
        var total = 0.0;
        for i in 0..10 {
            if i % 2 == 1 { continue; }
            if i > 6 { break; }
            total += i;
        }
        return total;
    }
    fn build() -> f32 {
        var xs = [3.0, 1.0];
        xs.push(4.0);
        xs.insert(0, 9.0);
        xs[1] += 10.0;
        let last = xs.pop();
        let first = xs.remove_at(0);
        return xs[0] * 100.0 + xs.length * 10.0 + last + first;
    }
    fn keep(v: f32) -> f32 {
        held.push(v);
        this.held.push(v * 2.0);
        return held.length;
    }
    fn copies() -> f32 {
        var a = [1.0];
        var b = a;
        b.push(2.0);
        return a.length * 10.0 + b.length;
    }
    fn asks(v: String) -> f32 {
        if names.contains(v) { return names.index_of(v); }
        return -1.0;
    }
    fn walk_while_changing() -> f32 {
        var xs = [1.0, 2.0];
        var seen = 0.0;
        for x in xs { xs.push(x); seen += 1.0; }
        return seen * 10.0 + xs.length;
    }
    fn empty_pop() -> f32 { var xs: List<f32> = []; return xs.pop(); }
    fn out_of_range() { var xs = [1.0]; xs[1] = 2.0; }
    fn fraction() { var xs = [1.0]; xs.remove_at(0.5); }
    fn forever() { var xs: List<f32> = []; while true { xs.push(1.0); } }
    fn clears() -> f32 { var xs = [1.0, 2.0]; xs.clear(); return xs.length; }
}"#;

fn call(
    runtime: &mut Runtime<'_, EmptyHost>,
    function: &str,
    args: Vec<Value>,
) -> Result<Value, RuntimeError> {
    runtime.call("Bag", function, args)
}

#[test]
fn ranges_and_lists_do_what_they_say() {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    assert_eq!(
        call(&mut runtime, "sum_to", vec![Value::Number(4.0)]),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        call(&mut runtime, "sum_to", vec![Value::Number(0.0)]),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        call(&mut runtime, "skip_odd", vec![]),
        Ok(Value::Number(12.0))
    );
    // [9, 13, 1, 4] -> pop 4 -> remove 9 -> [13, 1]
    assert_eq!(
        call(&mut runtime, "build", vec![]),
        Ok(Value::Number(1300.0 + 20.0 + 4.0 + 9.0))
    );
    assert_eq!(
        call(&mut runtime, "copies", vec![]),
        Ok(Value::Number(12.0))
    );
    assert_eq!(
        call(&mut runtime, "asks", vec![Value::String("b".into())]),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        call(&mut runtime, "asks", vec![Value::String("z".into())]),
        Ok(Value::Number(-1.0))
    );
    // The walk sees the list as it was when it began.
    assert_eq!(
        call(&mut runtime, "walk_while_changing", vec![]),
        Ok(Value::Number(24.0))
    );
    assert_eq!(call(&mut runtime, "clears", vec![]), Ok(Value::Number(0.0)));
}

#[test]
fn a_field_list_keeps_what_is_pushed_between_calls() {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut bag = runtime.instantiate("Bag").expect("instance");
    runtime
        .call_instance(&mut bag, "keep", vec![Value::Number(1.0)])
        .expect("keeps");
    assert_eq!(
        runtime.call_instance(&mut bag, "keep", vec![Value::Number(3.0)]),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        bag.field("held"),
        Some(&Value::array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(6.0)
        ]))
    );
}

#[test]
fn a_list_refuses_what_it_cannot_do() {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    assert_eq!(
        call(&mut runtime, "empty_pop", vec![]),
        Err(RuntimeError::IndexOutOfRange {
            index: 0,
            length: 0
        })
    );
    assert_eq!(
        call(&mut runtime, "out_of_range", vec![]),
        Err(RuntimeError::IndexOutOfRange {
            index: 1,
            length: 1
        })
    );
    assert_eq!(
        call(&mut runtime, "fraction", vec![]),
        Err(RuntimeError::IndexNotWhole(0.5))
    );
    assert_eq!(
        call(&mut runtime, "forever", vec![]),
        Err(RuntimeError::ListTooLong { limit: LIST_LIMIT })
    );
}
