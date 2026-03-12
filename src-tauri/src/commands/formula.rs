use std::collections::HashMap;
use std::sync::Arc;

use crate::providers::SharedProviderData;
use kustom_formula::{evaluate, EvalContext};
use tauri::State;

#[tauri::command]
pub async fn evaluate_formula(
    formula: String,
    globals: HashMap<String, String>,
    provider_data: State<'_, SharedProviderData>,
) -> Result<String, String> {
    let mut ctx = EvalContext::new();

    // Set globals and write them into shared data so providers can read them
    {
        let mut data = provider_data.write().await;
        let gv_map = data.entry("gv".to_string()).or_insert_with(HashMap::new);
        gv_map.clear();
        for (k, v) in &globals {
            gv_map.insert(k.clone(), v.clone());
        }
    }
    for (k, v) in globals {
        ctx.globals
            .insert(k, kustom_formula::value::Value::Text(v));
    }

    // Set provider data
    let data: tokio::sync::RwLockReadGuard<'_, std::collections::HashMap<String, std::collections::HashMap<String, String>>> = provider_data.read().await;
    let mut providers = HashMap::new();
    for (prefix, fields) in data.iter() {
        let mut provider_map = HashMap::new();
        for (field, value) in fields {
            provider_map.insert(
                field.clone(),
                kustom_formula::value::Value::Text(value.clone()),
            );
        }
        providers.insert(prefix.clone(), provider_map);
    }
    ctx.providers = Arc::new(providers);

    Ok(evaluate(&formula, &ctx))
}

#[tauri::command]
pub async fn get_provider_data(
    provider_data: State<'_, SharedProviderData>,
) -> Result<HashMap<String, HashMap<String, String>>, String> {
    let data = provider_data.read().await;
    Ok(data.clone())
}
