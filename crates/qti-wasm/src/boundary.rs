//! Fallible JavaScript deserialization occurs inside the adapter, never in the ABI.

use js_sys::{Array, Date, Reflect, Uint8Array};
use tsify::{Ts, Tsify};
use wasm_bindgen::{JsCast, JsError, JsValue, prelude::wasm_bindgen};

use crate::adapter::{self, MAX_ENTRY_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use crate::{
    CheckPackageResult, ConvertRequest, ConvertResult, Diagnostic, FormatInventory, PackageInput,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Object, js_name = keys)]
    fn own_keys(value: &JsValue) -> Result<Array, JsValue>;
}

/// Returns the current shared-registry format inventory or a serialization exception.
#[wasm_bindgen]
pub fn formats() -> Result<Ts<FormatInventory>, JsError> {
    adapter::format_inventory()
        .into_ts()
        .map_err(|error| JsError::new(&error.to_string()))
}

/// Copies JavaScript input into Rust ownership and returns expected failures as result values.
///
/// Resolves the UTC date once when the request omits it; serialization failures can throw.
#[wasm_bindgen]
pub fn convert(request: Ts<ConvertRequest>) -> Result<Ts<ConvertResult>, JsError> {
    let result = match conversion_preflight(&request.js_value()).and_then(|()| {
        request
            .to_rust()
            .map_err(|error| Box::new(Diagnostic::request("invalidRequest", error.to_string())))
    }) {
        Ok(request) => {
            // Resolve host time once; shared engines receive only explicit metadata.
            let date = Date::new_0()
                .to_iso_string()
                .as_string()
                .unwrap_or_default();
            adapter::convert_request(request, date.get(..10).unwrap_or(""))
        }
        Err(error) => ConvertResult::Error {
            error: *error,
            warnings: Vec::new(),
        },
    };
    result
        .into_ts()
        .map_err(|error| JsError::new(&error.to_string()))
}

/// Inspects owned ZIP or entry bytes and returns invalid transport as `CheckPackageResult::Error`.
///
/// Serialization failures can throw; malformed package content becomes checker findings.
#[wasm_bindgen(js_name = checkPackage)]
pub fn check_package(input: Ts<PackageInput>) -> Result<Ts<CheckPackageResult>, JsError> {
    let result = match package_preflight(&input.js_value()).and_then(|()| {
        input
            .to_rust()
            .map_err(|error| Box::new(Diagnostic::request("invalidRequest", error.to_string())))
    }) {
        Ok(input) => adapter::check_package_request(input),
        Err(error) => CheckPackageResult::Error { error: *error },
    };
    result
        .into_ts()
        .map_err(|error| JsError::new(&error.to_string()))
}

fn get(value: &JsValue, key: &str) -> Result<JsValue, Box<Diagnostic>> {
    Reflect::get(value, &JsValue::from_str(key)).map_err(|_| {
        Box::new(Diagnostic::request(
            "invalidRequest",
            format!("cannot read {key}"),
        ))
    })
}

fn check_fields(value: &JsValue, context: &str, allowed: &[&str]) -> Result<(), Box<Diagnostic>> {
    if !value.is_object() || value.is_null() || Array::is_array(value) {
        return Err(Box::new(Diagnostic::request(
            "invalidRequest",
            format!("{context} must be an object"),
        )));
    }
    let keys = own_keys(value).map_err(|_| {
        Box::new(Diagnostic::request(
            "invalidRequest",
            format!("cannot inspect fields in {context}"),
        ))
    })?;
    // serde-wasm-bindgen's struct reader ignores extras even with deny_unknown_fields.
    // Keep these boundary field names aligned with the Rust transport declarations.
    for key in keys.iter() {
        let key = key.as_string().ok_or_else(|| {
            Box::new(Diagnostic::request(
                "invalidRequest",
                format!("cannot inspect fields in {context}"),
            ))
        })?;
        if !allowed.contains(&key.as_str()) {
            return Err(Box::new(Diagnostic::request(
                "invalidRequest",
                format!("unknown field {key} in {context}"),
            )));
        }
    }
    Ok(())
}

fn bytes_size(value: &JsValue, limit: usize) -> Result<usize, Box<Diagnostic>> {
    // ASVS 1.5.2 / 2.2.1 / 5.2.1: check the actual typed-array length before serde copies bytes.
    let bytes = value.dyn_ref::<Uint8Array>().ok_or_else(|| {
        Box::new(Diagnostic::request(
            "invalidRequest",
            "bytes must be a Uint8Array",
        ))
    })?;
    let size = bytes.length() as usize;
    adapter::bound_size(size, limit)?;
    Ok(size)
}

fn files_size(value: &JsValue, context: &str) -> Result<usize, Box<Diagnostic>> {
    if !Array::is_array(value) {
        return Err(Box::new(Diagnostic::request(
            "invalidRequest",
            "entries and companions must be arrays",
        )));
    }
    let files = value.unchecked_ref::<Array>();
    if files.length() as usize > MAX_FILES {
        return Err(Box::new(Diagnostic::request(
            "inputLimit",
            "input exceeds 10000 files",
        )));
    }
    let mut total = 0;
    for index in 0..files.length() {
        let file = files.get(index);
        check_fields(&file, &format!("{context}[{index}]"), &["name", "bytes"])?;
        check_string(&file, "name", 4096)?;
        total += bytes_size(&get(&file, "bytes")?, MAX_ENTRY_BYTES)?;
        adapter::bound_size(total, MAX_TOTAL_BYTES)?;
    }
    Ok(total)
}

fn check_string(value: &JsValue, key: &str, limit: usize) -> Result<(), Box<Diagnostic>> {
    let field = get(value, key)?;
    if field.is_undefined() || field.is_null() {
        return Ok(());
    }
    if field.is_string() {
        let string = js_sys::JsString::from(field);
        if string.length() as usize > limit {
            return Err(Box::new(Diagnostic::request(
                "inputLimit",
                format!("{key} exceeds {limit} characters"),
            )));
        }
    }
    Ok(())
}

fn conversion_preflight(request: &JsValue) -> Result<(), Box<Diagnostic>> {
    check_fields(
        request,
        "request",
        &[
            "inputFormat",
            "outputFormat",
            "input",
            "allowMixed",
            "limit",
            "outputName",
            "document",
            "shuffleSeed",
        ],
    )?;
    check_string(request, "inputFormat", 128)?;
    check_string(request, "outputFormat", 128)?;
    check_string(request, "outputName", 4096)?;
    let document = get(request, "document")?;
    if !document.is_undefined() && !document.is_null() {
        check_fields(&document, "request.document", &["title", "date"])?;
        check_string(&document, "title", 65_536)?;
        check_string(&document, "date", 10)?;
    }
    let input = get(request, "input")?;
    check_string(&input, "name", 4096)?;
    match get(&input, "kind")?.as_string().as_deref() {
        Some("file") => {
            check_fields(
                &input,
                "request.input",
                &["kind", "name", "bytes", "companions"],
            )?;
            let total = bytes_size(&get(&input, "bytes")?, MAX_TOTAL_BYTES)?;
            let companions = get(&input, "companions")?;
            let companions_total = if companions.is_undefined() {
                0
            } else {
                files_size(&companions, "request.input.companions")?
            };
            adapter::bound_size(total + companions_total, MAX_TOTAL_BYTES)
        }
        Some("entries") => {
            check_fields(&input, "request.input", &["kind", "name", "entries"])?;
            files_size(&get(&input, "entries")?, "request.input.entries").map(|_| ())
        }
        _ => Err(Box::new(Diagnostic::request(
            "invalidRequest",
            "input kind must be file or entries",
        ))),
    }
}

fn package_preflight(input: &JsValue) -> Result<(), Box<Diagnostic>> {
    match get(input, "kind")?.as_string().as_deref() {
        Some("zip") => {
            check_fields(input, "package", &["kind", "bytes"])?;
            bytes_size(&get(input, "bytes")?, MAX_TOTAL_BYTES).map(|_| ())
        }
        Some("entries") => {
            check_fields(input, "package", &["kind", "entries"])?;
            files_size(&get(input, "entries")?, "package.entries").map(|_| ())
        }
        _ => Err(Box::new(Diagnostic::request(
            "invalidRequest",
            "package kind must be zip or entries",
        ))),
    }
}
