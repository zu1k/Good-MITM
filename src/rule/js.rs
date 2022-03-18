use anyhow::{anyhow, Result};
use quick_js::{console::LogConsole, Context, JsValue};

pub fn js_eval(code: &str) -> Result<()> {
    let context = Context::builder().console(LogConsole).build()?;
    context.add_callback("runtime", callback_js_runtime)?;

    context.eval(code)?;
    Ok(())
}

pub fn callback_js_runtime() -> String {
    "QuickJS".into()
}

#[test]
fn test_js_eval() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    js_eval(r#"console.log("JavaScript Runtime: " + runtime())"#).unwrap();

    let context = Context::builder().console(LogConsole).build().unwrap();

    let value = context.eval("1 + 2").unwrap();
    assert_eq!(value, JsValue::Int(3));

    let value = context
        .eval_as::<String>(" var x = 100 + 250; x.toString() ")
        .unwrap();
    assert_eq!(&value, "350");

    // Callbacks.

    context
        .add_callback("myCallback", |a: i32, b: i32| a + b)
        .unwrap();

    context
        .eval(
            r#"
        // x will equal 30
        var x = myCallback(10, 20);
        "#,
        )
        .unwrap();
}
