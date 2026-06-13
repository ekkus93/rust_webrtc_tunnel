mod c_abi;
mod runtime;

use c_abi::*;

use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};
use serde::Serialize;

pub use runtime::{
    AndroidForwardRuntimeStatus, AndroidRuntimeStatus, AndroidTunnelController, AndroidTunnelMode,
    AndroidValidationResult,
};

#[derive(Serialize)]
pub(crate) struct IdentityValidationResult {
    pub(crate) valid: bool,
    pub(crate) message: Option<String>,
    pub(crate) canonical_public_identity: Option<String>,
    pub(crate) canonical_private_identity: Option<String>,
    pub(crate) peer_id: Option<String>,
}

pub(crate) fn into_c_string(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(_) => CString::new("ffi string contained interior NUL")
            .expect("static fallback string is valid")
            .into_raw(),
    }
}

pub(crate) fn with_controller<R>(
    handle: *mut AndroidTunnelController,
    f: impl FnOnce(&AndroidTunnelController) -> R,
) -> Result<R, String> {
    if handle.is_null() {
        return Err("runtime handle was null".to_owned());
    }
    // SAFETY: the pointer comes from `p2ptunnel_create_runtime` and remains owned by the caller.
    let controller = unsafe { &*handle };
    Ok(f(controller))
}

pub(crate) fn catch_api<F>(f: F) -> i32
where
    F: FnOnce() -> Result<(), String>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => 0,
        Ok(Err(_)) => -1,
        Err(_) => -2,
    }
}

pub(crate) fn catch_api_string<F>(f: F) -> *mut c_char
where
    F: FnOnce() -> Result<String, String>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => into_c_string(value),
        Ok(Err(error)) => into_c_string(error),
        Err(_) => into_c_string("panic while handling Android bridge call".to_owned()),
    }
}

pub(crate) fn to_jstring(env: &mut JNIEnv<'_>, value: String) -> jstring {
    env.new_string(value).map(|value| value.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeCreateRuntime(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jlong {
    p2ptunnel_create_runtime() as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeDestroyRuntime(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    unsafe { p2ptunnel_destroy_runtime(handle as *mut AndroidTunnelController) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeStartOffer(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    config_path: JString<'_>,
) -> jint {
    let config_path = match env.get_string(&config_path) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => return -1,
    };
    let c_path = match CString::new(config_path) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    match unsafe { p2ptunnel_start_offer(handle as *mut AndroidTunnelController, c_path.as_ptr()) }
    {
        0 => 0,
        value => value as jint,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeStartOfferWithIdentity(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    config_path: JString<'_>,
    identity_bytes: jni::objects::JByteArray<'_>,
) -> jint {
    let config_path = match env.get_string(&config_path) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => return -1,
    };
    let c_path = match CString::new(config_path) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let identity = match env.convert_byte_array(&identity_bytes) {
        Ok(bytes) => bytes,
        Err(_) => return -1,
    };
    match unsafe {
        p2ptunnel_start_offer_with_identity(
            handle as *mut AndroidTunnelController,
            c_path.as_ptr(),
            identity.as_ptr(),
            identity.len(),
        )
    } {
        0 => 0,
        value => value as jint,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeStartAnswer(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    config_path: JString<'_>,
) -> jint {
    let config_path = match env.get_string(&config_path) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => return -1,
    };
    let c_path = match CString::new(config_path) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    match unsafe { p2ptunnel_start_answer(handle as *mut AndroidTunnelController, c_path.as_ptr()) }
    {
        0 => 0,
        value => value as jint,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeStop(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jint {
    unsafe { p2ptunnel_stop(handle as *mut AndroidTunnelController) as jint }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeStatusJson(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jstring {
    let ptr = unsafe { p2ptunnel_status_json(handle as *mut AndroidTunnelController) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: the pointer was allocated by `p2ptunnel_status_json`.
    let value = unsafe { CString::from_raw(ptr) }.into_string().unwrap_or_else(|_| "{}".to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeRecentLogsJson(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    max_events: jint,
) -> jstring {
    let ptr = unsafe {
        p2ptunnel_recent_logs_json(handle as *mut AndroidTunnelController, max_events as usize)
    };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: the pointer was allocated by `p2ptunnel_recent_logs_json`.
    let value = unsafe { CString::from_raw(ptr) }.into_string().unwrap_or_else(|_| "[]".to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_RustValidationBridge_nativeValidateConfig(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    config_path: JString<'_>,
) -> jstring {
    let config_path = match env.get_string(&config_path) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let ptr = match CString::new(config_path) {
        Ok(c_path) => unsafe { p2ptunnel_validate_config(c_path.as_ptr()) },
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: the pointer was allocated by `p2ptunnel_validate_config`.
    let value = unsafe { CString::from_raw(ptr) }
        .into_string()
        .unwrap_or_else(|_| r#"{"valid":false,"message":"invalid utf-8"}"#.to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_RustValidationBridge_nativeValidateConfigWithIdentity(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    config_path: JString<'_>,
    identity_bytes: jni::objects::JByteArray<'_>,
) -> jstring {
    let config_path = match env.get_string(&config_path) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let identity = match env.convert_byte_array(&identity_bytes) {
        Ok(bytes) => bytes,
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let c_path = match CString::new(config_path) {
        Ok(value) => value,
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let ptr = unsafe {
        p2ptunnel_validate_config_with_identity(c_path.as_ptr(), identity.as_ptr(), identity.len())
    };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointer allocated by `p2ptunnel_validate_config_with_identity`.
    let value = unsafe { CString::from_raw(ptr) }
        .into_string()
        .unwrap_or_else(|_| r#"{"valid":false,"message":"invalid utf-8"}"#.to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_RustValidationBridge_nativeValidatePrivateIdentity(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    private_identity_toml: JString<'_>,
) -> jstring {
    let identity = match env.get_string(&private_identity_toml) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let c_identity = match CString::new(identity) {
        Ok(value) => value,
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let ptr = unsafe { p2ptunnel_validate_private_identity(c_identity.as_ptr()) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointer allocated by `p2ptunnel_validate_private_identity`.
    let value = unsafe { CString::from_raw(ptr) }
        .into_string()
        .unwrap_or_else(|_| r#"{"valid":false,"message":"invalid utf-8"}"#.to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_RustValidationBridge_nativeValidatePublicIdentity(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    public_identity_line: JString<'_>,
) -> jstring {
    let line = match env.get_string(&public_identity_line) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let c_line = match CString::new(line) {
        Ok(value) => value,
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let ptr = unsafe { p2ptunnel_validate_public_identity(c_line.as_ptr()) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointer allocated by `p2ptunnel_validate_public_identity`.
    let value = unsafe { CString::from_raw(ptr) }
        .into_string()
        .unwrap_or_else(|_| r#"{"valid":false,"message":"invalid utf-8"}"#.to_owned());
    to_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_RustValidationBridge_nativeGenerateIdentity(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    peer_id: JString<'_>,
) -> jstring {
    let peer_id = match env.get_string(&peer_id) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let c_peer_id = match CString::new(peer_id) {
        Ok(value) => value,
        Err(error) => {
            return to_jstring(
                &mut env,
                serde_json::json!({"valid": false, "message": error.to_string()}).to_string(),
            );
        }
    };
    let ptr = unsafe { p2ptunnel_generate_identity(c_peer_id.as_ptr()) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointer allocated by `p2ptunnel_generate_identity`.
    let value = unsafe { CString::from_raw(ptr) }
        .into_string()
        .unwrap_or_else(|_| r#"{"valid":false,"message":"invalid utf-8"}"#.to_owned());
    to_jstring(&mut env, value)
}

pub(crate) fn last_error_for_handle(handle: *mut AndroidTunnelController) -> String {
    with_controller(handle, |controller| controller.last_error())
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown error".to_owned())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeLastError(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jstring {
    let error = last_error_for_handle(handle as *mut AndroidTunnelController);
    to_jstring(&mut env, error)
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use p2p_crypto::generate_identity;
    use serde_json::Value;

    use super::*;

    fn read_and_free(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null(), "bridge returned a null pointer");
        let value = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
        unsafe { p2ptunnel_free_string(ptr) };
        value
    }

    #[test]
    fn destroy_runtime_handles_null_pointer() {
        unsafe {
            p2ptunnel_destroy_runtime(std::ptr::null_mut());
        }
    }

    #[test]
    fn destroy_runtime_is_safe_for_fresh_handle() {
        let handle = p2ptunnel_create_runtime();
        unsafe {
            p2ptunnel_destroy_runtime(handle);
        }
    }

    #[test]
    fn status_json_shape_is_stable_and_parseable() {
        let handle = p2ptunnel_create_runtime();
        let raw = unsafe { p2ptunnel_status_json(handle) };
        let parsed: Value = serde_json::from_str(&read_and_free(raw)).expect("status json");
        assert_eq!(parsed.get("state").and_then(Value::as_str), Some("stopped"));
        assert!(parsed.get("active").and_then(Value::as_bool).is_some());
        unsafe { p2ptunnel_destroy_runtime(handle) };
    }

    #[test]
    fn recent_logs_json_is_stable_and_side_effect_free() {
        let handle = p2ptunnel_create_runtime();
        let first: Value =
            serde_json::from_str(&read_and_free(unsafe { p2ptunnel_recent_logs_json(handle, 4) }))
                .expect("first logs json");
        let second: Value =
            serde_json::from_str(&read_and_free(unsafe { p2ptunnel_recent_logs_json(handle, 4) }))
                .expect("second logs json");
        assert_eq!(first, second);
        assert!(first.as_array().is_some());
        unsafe { p2ptunnel_destroy_runtime(handle) };
    }

    #[test]
    fn generate_identity_returns_expected_json_fields() {
        let peer_id = CString::new("android-test").expect("peer id cstring");
        let raw = unsafe { p2ptunnel_generate_identity(peer_id.as_ptr()) };
        let parsed: Value = serde_json::from_str(&read_and_free(raw)).expect("identity json");
        assert_eq!(parsed.get("valid").and_then(Value::as_bool), Some(true));
        assert!(parsed.get("peer_id").and_then(Value::as_str).is_some());
        assert!(parsed.get("canonical_public_identity").and_then(Value::as_str).is_some());
        assert!(parsed.get("canonical_private_identity").and_then(Value::as_str).is_some());
    }

    #[test]
    fn generate_identity_reports_invalid_utf8_peer_id_input() {
        let bytes = [0xFF_u8, 0_u8];
        let ptr = bytes.as_ptr() as *const c_char;
        let message = read_and_free(unsafe { p2ptunnel_generate_identity(ptr) });
        assert!(message.contains("peer_id was not valid UTF-8"));
    }

    #[test]
    fn validate_config_with_identity_rejects_invalid_utf8_identity_bytes() {
        let config_path = CString::new("/definitely/missing/config.toml").expect("config cstring");
        let identity_bytes = [0xFF_u8];
        let message = read_and_free(unsafe {
            p2ptunnel_validate_config_with_identity(
                config_path.as_ptr(),
                identity_bytes.as_ptr(),
                identity_bytes.len(),
            )
        });
        assert!(message.contains("identity bytes were not valid UTF-8"));
    }

    #[test]
    fn validate_config_with_identity_missing_config_returns_failure_payload() {
        let config_path = CString::new("/definitely/missing/config.toml").expect("config cstring");
        let identity = generate_identity("android-test").expect("identity");
        let identity_toml = identity.identity.render_toml();
        let raw = unsafe {
            p2ptunnel_validate_config_with_identity(
                config_path.as_ptr(),
                identity_toml.as_bytes().as_ptr(),
                identity_toml.len(),
            )
        };
        let parsed: Value = serde_json::from_str(&read_and_free(raw)).expect("validation json");
        assert_eq!(parsed.get("valid").and_then(Value::as_bool), Some(false));
        assert!(parsed.get("message").is_some());
    }

    #[test]
    fn last_error_path_reports_unknown_then_runtime_error() {
        let handle = p2ptunnel_create_runtime();
        assert_eq!(super::last_error_for_handle(handle), "unknown error");

        let config_path = CString::new("/definitely/missing/config.toml").expect("config cstring");
        assert_eq!(unsafe { p2ptunnel_start_offer(handle, config_path.as_ptr()) }, -1);

        let last_error = super::last_error_for_handle(handle);
        assert_ne!(last_error, "unknown error");
        let status: Value =
            serde_json::from_str(&read_and_free(unsafe { p2ptunnel_status_json(handle) }))
                .expect("status json after error");
        assert!(status.get("state").is_some());
        let generate = serde_json::from_str::<Value>(&read_and_free(unsafe {
            p2ptunnel_generate_identity(CString::new("android-test").expect("peer id").as_ptr())
        }))
        .expect("identity after error");
        assert_eq!(generate.get("valid").and_then(Value::as_bool), Some(true));

        unsafe { p2ptunnel_destroy_runtime(handle) };
    }

    #[test]
    fn null_runtime_handle_returns_error_message_for_status_json() {
        let message = read_and_free(unsafe { p2ptunnel_status_json(std::ptr::null_mut()) });
        assert!(message.contains("runtime handle was null"));
    }
}
