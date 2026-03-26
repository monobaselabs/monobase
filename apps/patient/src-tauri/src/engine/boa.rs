use boa_engine::{
    js_string, native_function::NativeFunction, object::builtins::JsArray,
    object::ObjectInitializer, property::Attribute, Context, JsArgs, JsError,
    JsNativeError, JsResult, JsValue, Source,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};

use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest as Sha256Digest};
use sha1::Sha1;

use flate2::read::GzDecoder;
use std::io::Read as IoRead;

use crate::db::Database;
use super::{EngineResponse, JsEngine};

const BUNDLE_JS_GZ: &[u8] = include_bytes!("../../js/bundle.js.gz");

/// Per-request log buffer — captures JS console output so it can be forwarded to the frontend.
type LogBuffer = Rc<RefCell<Vec<String>>>;

/// Decompress the gzip-compressed bundle at startup.
fn decompress_bundle() -> Result<String, String> {
    let mut decoder = GzDecoder::new(BUNDLE_JS_GZ);
    let mut bundle = String::new();
    decoder.read_to_string(&mut bundle)
        .map_err(|e| format!("Failed to decompress bundle: {}", e))?;
    Ok(bundle)
}

/// Request sent to the JS thread
struct JsRequest {
    method: String,
    url: String,
    body: Option<String>,
    headers: Vec<(String, String)>,
    reply: mpsc::Sender<Result<EngineResponse, String>>,
}

pub struct BoaEngine {
    tx: mpsc::Sender<JsRequest>,
}

impl BoaEngine {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let db = Arc::new(Database::new(db_path)?);
        let (tx, rx) = mpsc::channel::<JsRequest>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        // Spawn a dedicated thread for the Boa context (Context is !Send)
        let db_clone = db.clone();
        std::thread::spawn(move || {
            let mut context = Context::default();
            let log_buffer: LogBuffer = Rc::new(RefCell::new(Vec::new()));

            if let Err(e) = setup_context(&mut context, &db_clone, &log_buffer) {
                log::error!("Failed to setup Boa context: {}", e);
                let _ = ready_tx.send(Err(e));
                return;
            }
            log::info!("Boa JS thread ready (bundle loaded)");
            let _ = ready_tx.send(Ok(()));

            // Process requests sequentially on the persistent context
            while let Ok(req) = rx.recv() {
                let result = execute_request(&mut context, &log_buffer, &req.method, &req.url, req.body.as_deref(), &req.headers);
                let _ = req.reply.send(result);
            }
            log::info!("Boa JS thread shutting down");
        });

        // Block until the bundle is fully loaded
        ready_rx.recv()
            .map_err(|e| format!("JS thread died during setup: {}", e))?
            .map_err(|e| format!("JS thread setup failed: {}", e))?;

        log::info!("BoaEngine initialized (bundle pre-loaded)");
        Ok(Self { tx })
    }
}

impl JsEngine for BoaEngine {
    fn handle_request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
        headers: Vec<(&str, &str)>,
    ) -> Result<EngineResponse, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let req = JsRequest {
            method: method.to_string(),
            url: url.to_string(),
            body: body.map(String::from),
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            reply: reply_tx,
        };
        self.tx.send(req).map_err(|e| format!("JS thread send failed: {}", e))?;
        reply_rx.recv().map_err(|e| format!("JS thread recv failed: {}", e))?
    }
}

/// Set up the persistent Boa context: register natives, load the bundle.
fn setup_context(context: &mut Context, db: &Arc<Database>, log_buffer: &LogBuffer) -> Result<(), String> {
    // ── Register console (outputs to log crate AND per-request buffer) ──
    let buf = log_buffer.clone();
    let console_log_fn = unsafe { NativeFunction::from_closure(move |_this, args, context| {
        let msg = args.iter()
            .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
            .collect::<Result<Vec<_>, _>>()?
            .join(" ");
        log::info!("[JS] {}", msg);
        if let Ok(mut logs) = buf.try_borrow_mut() {
            logs.push(msg);
        }
        Ok(JsValue::undefined())
    }) };

    let console = ObjectInitializer::new(context)
        .function(console_log_fn.clone(), js_string!("log"), 1)
        .function(console_log_fn.clone(), js_string!("info"), 1)
        .function(console_log_fn.clone(), js_string!("warn"), 1)
        .function(console_log_fn.clone(), js_string!("error"), 1)
        .function(console_log_fn.clone(), js_string!("debug"), 1)
        .build();

    context.register_global_property(js_string!("console"), console, Attribute::all())
        .map_err(|e| format!("Failed to register console: {}", e))?;

    // ── Register db_execute / db_select as __db object ─────────
    let db_rc: Rc<RefCell<Arc<Database>>> = Rc::new(RefCell::new(db.clone()));

    let db_for_exec = db_rc.clone();
    let exec_fn = unsafe { NativeFunction::from_closure(move |_this, args, context| {
        let sql = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let params = if let Some(arr) = args.get(1) {
            js_array_to_json(arr, context)?
        } else {
            vec![]
        };
        let db = db_for_exec.borrow();
        let result = db.execute(&sql, params)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(e)))?;
        json_to_js_value(&serde_json::to_value(result).unwrap(), context)
    }) };

    let db_for_sel = db_rc.clone();
    let sel_fn = unsafe { NativeFunction::from_closure(move |_this, args, context| {
        let sql = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let params = if let Some(arr) = args.get(1) {
            js_array_to_json(arr, context)?
        } else {
            vec![]
        };
        let db = db_for_sel.borrow();
        let rows = db.select(&sql, params)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(e)))?;
        json_to_js_value(&serde_json::Value::Array(rows), context)
    }) };

    let db_obj = ObjectInitializer::new(context)
        .function(exec_fn, js_string!("execute"), 2)
        .function(sel_fn, js_string!("select"), 2)
        .build();

    context.register_global_property(js_string!("__db"), db_obj, Attribute::all())
        .map_err(|e| format!("Failed to register __db: {}", e))?;

    // ── Register crypto natives as __crypto object ───────────────
    register_crypto_natives(context)?;

    // ── Register password hashing as __bcrypt object ─────────────
    register_bcrypt_natives(context)?;

    // ── Load the bundle (decompress gzip + eval shim + Hono + Better Auth IIFE)
    let bundle = decompress_bundle()?;
    context.eval(Source::from_bytes(bundle.as_bytes()))
        .map_err(|e| format!("Failed to load bundle.js: {}", e))?;

    log::info!("Bundle loaded ({:.0} KB decompressed from {:.0} KB gzip)",
        bundle.len() as f64 / 1024.0,
        BUNDLE_JS_GZ.len() as f64 / 1024.0);
    Ok(())
}

/// Execute a single request on the persistent context.
fn execute_request(
    context: &mut Context,
    log_buffer: &LogBuffer,
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &[(String, String)],
) -> Result<EngineResponse, String> {
    // Clear log buffer before this request
    if let Ok(mut logs) = log_buffer.try_borrow_mut() {
        logs.clear();
    }

    // Serialize headers as JSON using serde (safe, no manual escaping)
    let headers_map: std::collections::HashMap<&str, &str> = headers.iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let headers_json = serde_json::to_string(&headers_map)
        .unwrap_or_else(|_| "{}".to_string());

    // Set arguments as globals (avoids string escaping + dynamic code generation)
    let body_val = match body {
        Some(b) => JsValue::from(js_string!(b)),
        None => JsValue::null(),
    };

    context.register_global_property(js_string!("__arg_method"), JsValue::from(js_string!(method)), Attribute::all())
        .map_err(|e| format!("Failed to set __arg_method: {}", e))?;
    context.register_global_property(js_string!("__arg_url"), JsValue::from(js_string!(url)), Attribute::all())
        .map_err(|e| format!("Failed to set __arg_url: {}", e))?;
    context.register_global_property(js_string!("__arg_body"), body_val, Attribute::all())
        .map_err(|e| format!("Failed to set __arg_body: {}", e))?;
    context.register_global_property(js_string!("__arg_headers"), JsValue::from(js_string!(headers_json.as_str())), Attribute::all())
        .map_err(|e| format!("Failed to set __arg_headers: {}", e))?;

    // Single eval — calls the pre-registered dispatcher, no string escaping needed
    context.eval(Source::from_bytes(b"globalThis.__res = null; globalThis.__dispatch(__arg_method, __arg_url, __arg_body, __arg_headers)"))
        .map_err(|e| format!("Failed to execute dispatch: {}", e))?;

    let _ = context.run_jobs();

    // Drain log buffer for this request
    let logs = if let Ok(mut buf) = log_buffer.try_borrow_mut() {
        buf.drain(..).collect()
    } else {
        vec![]
    };

    // Read __res directly from the global object (avoids parsing JS source)
    let global = context.global_object();
    let res_val = global.get(js_string!("__res"), context)
        .map_err(|e| format!("Failed to read __res: {}", e))?;

    if let Some(s) = res_val.as_string() {
        let s = s.to_std_string_escaped();
        let parsed: serde_json::Value = serde_json::from_str(&s)
            .map_err(|e| format!("Failed to parse response JSON: {} — raw: {}", e, s))?;
        let status = parsed["s"].as_u64().unwrap_or(200) as u16;
        let body_str = parsed["b"].as_str().unwrap_or("null");
        let body: serde_json::Value = serde_json::from_str(body_str)
            .unwrap_or_else(|_| serde_json::Value::String(body_str.to_string()));
        let mut resp_headers = Vec::new();
        if let Some(h) = parsed["h"].as_object() {
            for (k, v) in h {
                if let Some(vs) = v.as_str() {
                    resp_headers.push((k.clone(), vs.to_string()));
                }
            }
        }
        Ok(EngineResponse { status, body, headers: resp_headers, logs })
    } else {
        Err(format!("No response from Hono (promise may not have resolved — __res was not set). Logs: {:?}", logs))
    }
}

fn register_crypto_natives(context: &mut Context) -> Result<(), String> {
    let rand_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let len = args.get_or_undefined(0).to_number(context)? as usize;
        let mut bytes = vec![0u8; len];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(format!("getrandom failed: {}", e))))?;
        let arr = JsArray::new(context);
        for (i, b) in bytes.iter().enumerate() {
            arr.set(i as u32, JsValue::from(*b as i32), false, context)?;
        }
        Ok(arr.into())
    });

    let uuid_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        Ok(JsValue::from(js_string!(uuid::Uuid::new_v4().to_string().as_str())))
    });

    let sha256_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = js_array_to_bytes(args.get_or_undefined(0), context)?;
        let hash = Sha256::digest(&input);
        let arr = JsArray::new(context);
        for (i, b) in hash.iter().enumerate() {
            arr.set(i as u32, JsValue::from(*b as i32), false, context)?;
        }
        Ok(arr.into())
    });

    let sha1_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = js_array_to_bytes(args.get_or_undefined(0), context)?;
        let hash = Sha1::digest(&input);
        let arr = JsArray::new(context);
        for (i, b) in hash.iter().enumerate() {
            arr.set(i as u32, JsValue::from(*b as i32), false, context)?;
        }
        Ok(arr.into())
    });

    let hmac_sign_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = js_array_to_bytes(args.get_or_undefined(0), context)?;
        let data = js_array_to_bytes(args.get_or_undefined(1), context)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&key)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(format!("HMAC init failed: {}", e))))?;
        mac.update(&data);
        let result = mac.finalize().into_bytes();
        let arr = JsArray::new(context);
        for (i, b) in result.iter().enumerate() {
            arr.set(i as u32, JsValue::from(*b as i32), false, context)?;
        }
        Ok(arr.into())
    });

    let hmac_verify_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = js_array_to_bytes(args.get_or_undefined(0), context)?;
        let sig = js_array_to_bytes(args.get_or_undefined(1), context)?;
        let data = js_array_to_bytes(args.get_or_undefined(2), context)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&key)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(format!("HMAC init failed: {}", e))))?;
        mac.update(&data);
        let valid = mac.verify_slice(&sig).is_ok();
        Ok(JsValue::from(valid))
    });

    let crypto_obj = ObjectInitializer::new(context)
        .function(rand_fn, js_string!("getRandomValues"), 1)
        .function(uuid_fn, js_string!("randomUUID"), 0)
        .function(sha256_fn, js_string!("sha256"), 1)
        .function(sha1_fn, js_string!("sha1"), 1)
        .function(hmac_sign_fn, js_string!("hmacSha256Sign"), 2)
        .function(hmac_verify_fn, js_string!("hmacSha256Verify"), 3)
        .build();

    context.register_global_property(js_string!("__crypto"), crypto_obj, Attribute::all())
        .map_err(|e| format!("Failed to register __crypto: {}", e))?;

    Ok(())
}

fn register_bcrypt_natives(context: &mut Context) -> Result<(), String> {
    let hash_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let password = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let hash = bcrypt::hash(&password, 10)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(format!("bcrypt hash failed: {}", e))))?;
        Ok(JsValue::from(js_string!(hash.as_str())))
    });

    let verify_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let password = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let hash = args.get_or_undefined(1).to_string(context)?.to_std_string_escaped();
        let valid = bcrypt::verify(&password, &hash).unwrap_or(false);
        Ok(JsValue::from(valid))
    });

    let bcrypt_obj = ObjectInitializer::new(context)
        .function(hash_fn, js_string!("hash"), 1)
        .function(verify_fn, js_string!("verify"), 2)
        .build();

    context.register_global_property(js_string!("__bcrypt"), bcrypt_obj, Attribute::all())
        .map_err(|e| format!("Failed to register __bcrypt: {}", e))?;

    Ok(())
}

// ── Helpers: JsValue <-> JSON conversion ──────────────────────────────

fn js_array_to_json(value: &JsValue, context: &mut Context) -> JsResult<Vec<serde_json::Value>> {
    if value.is_undefined() || value.is_null() {
        return Ok(vec![]);
    }
    let obj = value.as_object().ok_or_else(|| {
        JsError::from_native(JsNativeError::typ().with_message("Expected array"))
    })?;
    let arr = JsArray::from_object(obj.clone()).map_err(|_| {
        JsError::from_native(JsNativeError::typ().with_message("Expected array"))
    })?;
    let len = arr.length(context)?;
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        let item = arr.get(i, context)?;
        let json_val = js_value_to_json(&item, context)
            .map_err(|e| JsError::from_native(JsNativeError::error().with_message(e)))?;
        result.push(json_val);
    }
    Ok(result)
}

fn js_value_to_json(value: &JsValue, context: &mut Context) -> Result<serde_json::Value, String> {
    if value.is_undefined() || value.is_null() { return Ok(serde_json::Value::Null); }
    if let Some(b) = value.as_boolean() { return Ok(serde_json::Value::Bool(b)); }
    if let Some(n) = value.as_number() {
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            return Ok(serde_json::json!(n as i64));
        }
        return Ok(serde_json::json!(n));
    }
    if let Some(s) = value.as_string() {
        return Ok(serde_json::Value::String(s.to_std_string_escaped()));
    }
    if let Some(obj) = value.as_object() {
        if let Ok(arr) = JsArray::from_object(obj.clone()) {
            let len = arr.length(context).map_err(|e| e.to_string())?;
            let mut result = Vec::with_capacity(len as usize);
            for i in 0..len {
                let item = arr.get(i, context).map_err(|e| e.to_string())?;
                result.push(js_value_to_json(&item, context)?);
            }
            return Ok(serde_json::Value::Array(result));
        }
        let mut map = serde_json::Map::new();
        let keys = obj.own_property_keys(context).map_err(|e| e.to_string())?;
        for key in keys {
            let key_str = key.to_string();
            let prop_value = obj.get(key.clone(), context).map_err(|e| e.to_string())?;
            map.insert(key_str, js_value_to_json(&prop_value, context)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    Ok(serde_json::Value::Null)
}

/// Convert a JS array of numbers to a Vec<u8> (for crypto byte arrays)
fn js_array_to_bytes(value: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    if value.is_undefined() || value.is_null() {
        return Ok(vec![]);
    }
    let obj = value.as_object().ok_or_else(|| {
        JsError::from_native(JsNativeError::typ().with_message("Expected array of bytes"))
    })?;
    let arr = JsArray::from_object(obj.clone()).map_err(|_| {
        JsError::from_native(JsNativeError::typ().with_message("Expected array of bytes"))
    })?;
    let len = arr.length(context)?;
    let mut bytes = Vec::with_capacity(len as usize);
    for i in 0..len {
        let item = arr.get(i, context)?;
        let n = item.to_number(context)? as u8;
        bytes.push(n);
    }
    Ok(bytes)
}

fn json_to_js_value(value: &serde_json::Value, context: &mut Context) -> JsResult<JsValue> {
    match value {
        serde_json::Value::Null => Ok(JsValue::null()),
        serde_json::Value::Bool(b) => Ok(JsValue::from(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Ok(JsValue::from(i)) }
            else if let Some(f) = n.as_f64() { Ok(JsValue::from(f)) }
            else { Ok(JsValue::null()) }
        }
        serde_json::Value::String(s) => Ok(JsValue::from(js_string!(s.as_str()))),
        serde_json::Value::Array(arr) => {
            let js_arr = JsArray::new(context);
            for (i, item) in arr.iter().enumerate() {
                let js_item = json_to_js_value(item, context)?;
                js_arr.set(i as u32, js_item, false, context)?;
            }
            Ok(js_arr.into())
        }
        serde_json::Value::Object(obj) => {
            let js_obj = ObjectInitializer::new(context).build();
            for (key, val) in obj {
                let js_val = json_to_js_value(val, context)?;
                js_obj.set(js_string!(key.as_str()), js_val, false, context)?;
            }
            Ok(js_obj.into())
        }
    }
}
