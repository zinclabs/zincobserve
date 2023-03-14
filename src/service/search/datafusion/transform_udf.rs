// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use datafusion::{
    arrow::{
        array::{Array, ArrayRef, StringArray},
        datatypes::DataType,
    },
    logical_expr::{ScalarFunctionImplementation, ScalarUDF, Volatility},
    physical_plan::functions::make_scalar_function,
    prelude::create_udf,
};

use mlua::{Function, Lua, LuaSerdeExt, MultiValue};
use std::sync::Arc;

use crate::infra::config::QUERY_FUNCTIONS;

fn create_user_df(
    fn_name: &str,
    num_args: u8,
    pow_scalar: ScalarFunctionImplementation,
) -> ScalarUDF {
    let mut input_vec = vec![];
    for _i in 0..num_args {
        input_vec.push(DataType::Utf8);
    }

    create_udf(
        fn_name,
        input_vec,
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        pow_scalar,
    )
}

/* fn get_udf(fn_name: String, js_func: &str, num_args: u8) -> datafusion::logical_expr::ScalarUDF {
    let local_fn_name = fn_name.to_owned();
    let local_js_func = js_func.to_owned();

    let pow_calc = move |args: &[ArrayRef]| {
        // Create a new Isolate and make it the current one.
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        // Create a stack-allocated handle scope.
        let handle_scope = &mut v8::HandleScope::new(isolate);
        // Create a new context.
        let context = v8::Context::new(handle_scope);
        // Enter the context for compiling and running the hello world script.
        let mut scope = v8::ContextScope::new(handle_scope, context);

        let len = args[0].len();

        let mut data_vec = vec![];

        for i in 0..len {
            data_vec.insert(
                i,
                eval_user_fn(
                    &mut scope,
                    args,
                    i,
                    fn_name.to_owned().as_str(),
                    local_js_func.to_owned().as_str(),
                ),
            );
        }
        let result = StringArray::from(data_vec);

        // `Ok` because no error occurred during the calculation (we should add one if exponent was [0, 1[ and the base < 0 because that panics!)
        // `Arc` because arrays are immutable, thread-safe, trait objects.
        Ok(Arc::new(result) as ArrayRef)
    };
    // the function above expects an `ArrayRef`, but DataFusion may pass a scalar to a UDF.
    // thus, we use `make_scalar_function` to decorare the closure so that it can handle both Arrays and Scalar values.
    let pow_scalar = make_scalar_function(pow_calc);

    // Next:
    // * give it a name so that it shows nicely when the plan is printed
    // * declare what input it expects
    // * declare its return type
    let pow_udf = create_user_df(local_fn_name.as_str(), num_args, pow_scalar);
    pow_udf
}

fn eval_user_fn(
    scope: &mut ContextScope<HandleScope>,
    args: &[Arc<dyn Array>],
    stream: usize,
    fn_name: &str,
    js_func: &str,
) -> String {
    let mut js_fn_arg = String::new();
    js_fn_arg.push_str(fn_name);
    js_fn_arg.push('(');

    for (_i, arg) in args.iter().enumerate() {
        let col = arg
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("cast failed");
        if col.data_type().equals_datatype(&DataType::Utf8) {
            js_fn_arg.push_str(&format!("'{}'", col.value(stream)));
        } else {
            js_fn_arg.push_str(&col.value(stream));
        }
        js_fn_arg.push(',');
    }
    js_fn_arg.pop();
    js_fn_arg.push(')');

    let js_fn_str = format!(r#"{};{};"#, js_func, js_fn_arg);
    let source_reg = v8::String::new(scope, &js_fn_str).unwrap();
    // scope.add_context_data(context, source_log);
    let script = v8::Script::compile(scope, source_reg, None).unwrap();
    // Run the script to get the result.
    let result = script.run(scope);
    match result {
        Some(ret) => {
            let _result = ret.to_string(scope).unwrap();
            _result.to_rust_string_lossy(scope)
        }
        None => "".to_owned(),
    }
} */

pub async fn get_all_transform(org_id: &str) -> Vec<datafusion::logical_expr::ScalarUDF> {
    let mut udf;
    let mut udf_list = Vec::new();
    for trnasform in QUERY_FUNCTIONS.iter() {
        let key = trnasform.key();
        //do not register ingest_time transforms
        if trnasform.stream_name.is_empty() && key.contains(org_id) {
            //if transform.trans_type == 1 {
            udf = get_udf_lua(
                trnasform.name.to_owned(),
                trnasform.function.to_owned().as_str(),
                trnasform.num_args,
            );
            /* } else {

                udf = get_udf(
                    transform.name.to_owned(),
                    transform.function.to_owned().as_str(),
                    transform.num_args,
                );
            } */

            udf_list.push(udf);
        }
    }
    udf_list
}

fn get_udf_lua(
    fn_name: String,
    js_func: &str,
    num_args: u8,
) -> datafusion::logical_expr::ScalarUDF {
    let local_fn_name = fn_name;
    let local_js_func = js_func.to_owned();
    let pow_calc = move |args: &[ArrayRef]| {
        //Lua
        let lua = Lua::new();
        //Register Lua Function
        let func = load_tarnsform(&lua, local_js_func.to_string());
        let len = args[0].len();

        let mut data_vec = vec![];

        for i in 0..len {
            data_vec.insert(i, lua_transform(&lua, &func, args, i));
        }
        let result = StringArray::from(data_vec);

        // `Ok` because no error occurred during the calculation (we should add one if exponent was [0, 1[ and the base < 0 because that panics!)
        // `Arc` because arrays are immutable, thread-safe, trait objects.
        Ok(Arc::new(result) as ArrayRef)
    };
    // the function above expects an `ArrayRef`, but DataFusion may pass a scalar to a UDF.
    // thus, we use `make_scalar_function` to decorare the closure so that it can handle both Arrays and Scalar values.
    let pow_scalar = make_scalar_function(pow_calc);

    // Next:
    // * give it a name so that it shows nicely when the plan is printed
    // * declare what input it expects
    // * declare its return type
    let pow_udf = create_user_df(local_fn_name.as_str(), num_args, pow_scalar);
    pow_udf
}

fn load_tarnsform(lua: &Lua, js_func: String) -> Function {
    lua.load(&js_func).eval().unwrap()
}

fn lua_transform(lua: &Lua, func: &Function, args: &[ArrayRef], stream: usize) -> String {
    let mut input = MultiValue::new();

    for (_i, arg) in args.iter().enumerate() {
        let col = arg
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("cast failed");

        input.push_front(lua.to_value(&col.value(stream)).unwrap());
    }

    let _res = func.call::<_, String>(input);
    match _res {
        Ok(res) => res,
        Err(err) => {
            log::error!("lua_transform execute error: {}", err);
            "".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use datafusion::arrow::array::{Int64Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::datasource::MemTable;
    use datafusion::from_slice::FromSlice;
    use datafusion::prelude::SessionContext;
    use std::sync::Arc;

    #[tokio::test]
    async fn time_range() {
        //let data_time = parse_time("2021-01-01T00:00:00.000Z").unwrap();
        let sql = "select *, luaconcat(log,pod_id) as c from t ";

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("pod_id", DataType::Int64, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from_slice(&["a", "b", "c", "d"])),
                Arc::new(Int64Array::from_slice(&[1, 2, 1, 2])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark SQLsession
        let ctx = SessionContext::new();

        let udf = get_udf_lua("luaconcat".to_string(), "function(a, b) return a..b end", 2);
        ctx.register_udf(udf.clone());

        // declare a table in memory. In spark API, this corresponds to createDataFrame(...).
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        let df = ctx.sql(sql).await.unwrap();
        let result = df.collect().await.unwrap();
        /* datafusion::assert_batches_sorted_eq!(
            vec![
                "+-----+--------+----+",
                "| log | pod_id | c  |",
                "+-----+--------+----+",
                "| a   | 1      | 1a |",
                "| b   | 2      | 2b |",
                "| c   | 1      | 1c |",
                "| d   | 2      | 2d |",
                "+-----+--------+----+",
            ],
            &df
        ); */
        let count = result.iter().map(|batch| batch.num_rows()).sum::<usize>();
        assert_eq!(count, 4);
    }
}
