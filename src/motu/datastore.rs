use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use reqwest::blocking::Client;
use serde_json::Value;

enum JsonValue {
    String(String),
    Number(f64),
    Bool(bool),
}

impl Display for JsonValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonValue::String(s) => write!(f, "{}", s),
            JsonValue::Number(n) => write!(f, "{}", n),
            JsonValue::Bool(b) => write!(f, "{}", b),
        }
    }
}

enum MotuMsg {}

struct HttpDatastore {
    client: Client,
    base_url: String,
    input: Receiver<MotuMsg>,
    output: Sender<MotuMsg>,
    int_callbacks: Arc<Mutex<BTreeMap<String, Vec<Box<dyn Fn(i64) + Send>>>>>,
    float_callbacks: Arc<Mutex<BTreeMap<String, Vec<Box<dyn Fn(f64) + Send>>>>>,
    string_callbacks: Arc<Mutex<BTreeMap<String, Vec<Box<dyn Fn(String) + Send>>>>>,
    bool_callbacks: Arc<Mutex<BTreeMap<String, Vec<Box<dyn Fn(bool) + Send>>>>>,
    cache: Arc<Mutex<BTreeMap<String, Value>>>,
}

impl HttpDatastore {
    fn new(base_url: &str, input: Receiver<MotuMsg>, output: Sender<MotuMsg>) -> Self {
        HttpDatastore {
            client: Client::new(),
            base_url: base_url.to_string(),
            input,
            output,
            int_callbacks: Arc::new(Mutex::new(BTreeMap::new())),
            float_callbacks: Arc::new(Mutex::new(BTreeMap::new())),
            string_callbacks: Arc::new(Mutex::new(BTreeMap::new())),
            bool_callbacks: Arc::new(Mutex::new(BTreeMap::new())),
            cache: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    fn poll(&mut self) {
        // CAVEAT: this is basically a transliteration of the old datstore.go code. That code
        // seemed to work but obviously wasn't rusty.
        let mut etag = 0;
        loop {
            let response = self
                .client
                .get(&self.base_url)
                .header("If-None-Match", etag.to_string())
                .send();
            if let Ok(response) = response {
                if response.status().as_u16() == 304 {
                    // No changs
                    // TODO: are we supposed to wait before the next poll? Consult api_spec.md.
                    continue;
                }
                etag = response
                    .headers()
                    .get("ETag")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(etag);
                // This now has all the new data from the server.
                let data = response.json::<BTreeMap<String, Value>>().unwrap();
                println!("Received data from store: {:?}", data);
                // Process any data for which we have a callback registered.
                // For each key with a callback, check if the value has changed from the cache (or
                // if not in the cache, treat as changed). If changed, update the cache and call
                // the callback.
                //
                // Note: we intentionally do not cache values for which we have no callback so that
                // if we add a new callback, we ensure we always call the callback on next poll,
                // even if the value hasn't changed since we last polled. (We assume that new
                // bindings should be greedy).
                self.int_callbacks
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .for_each(|(key, cbs)| {
                        if let Some(Value::Number(num)) = data.get(key) {
                            if let Some(i) = num.as_i64() {
                                let old_value = self
                                    .cache
                                    .lock()
                                    .unwrap()
                                    .insert(key.clone(), Value::Number(num.clone()));
                                if old_value != Some(Value::Number(num.clone())) {
                                    for cb in cbs.iter_mut() {
                                        cb(i);
                                    }
                                }
                            }
                        }
                    });
                self.float_callbacks
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .for_each(|(key, cbs)| {
                        if let Some(Value::Number(num)) = data.get(key) {
                            if let Some(f) = num.as_f64() {
                                let old_value = self
                                    .cache
                                    .lock()
                                    .unwrap()
                                    .insert(key.clone(), Value::Number(num.clone()));
                                if old_value != Some(Value::Number(num.clone())) {
                                    for cb in cbs.iter_mut() {
                                        cb(f);
                                    }
                                }
                            }
                        }
                    });
                self.string_callbacks
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .for_each(|(key, cbs)| {
                        if let Some(Value::String(s)) = data.get(key) {
                            let old_value = self
                                .cache
                                .lock()
                                .unwrap()
                                .insert(key.clone(), Value::String(s.clone()));
                            if old_value != Some(Value::String(s.clone())) {
                                for cb in cbs.iter_mut() {
                                    cb(s.clone());
                                }
                            }
                        }
                    });
                self.bool_callbacks
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .for_each(|(key, cbs)| {
                        if let Some(Value::Bool(b)) = data.get(key) {
                            let old_value = self
                                .cache
                                .lock()
                                .unwrap()
                                .insert(key.clone(), Value::Bool(*b));
                            if old_value != Some(Value::Bool(*b)) {
                                for cb in cbs.iter_mut() {
                                    cb(*b);
                                }
                            }
                        }
                    });
            }
        }
    }

    fn set(&self, key: &str, value: JsonValue) {
        self.client
            .patch(&self.base_url)
            .header("Content Type", "application/x-www-form-urlencoded")
            .header("Accept-Encoding", "*/*")
            .body(format!("json={{\"{}\":\"{}\"}}", key, value))
            .send()
            .unwrap();
    }

    fn get(&self, key: &str) -> Option<Value> {
        self.cache.lock().unwrap().get(key).cloned()
    }
}
