//! Android JNI bridge (`Java_*`). These `#[no_mangle] extern "system"` functions
//! marshal JNI arguments (handles, jstrings, byte arrays) and delegate to the
//! C-ABI surface in [`crate::c_abi`], returning results as Java primitives or
//! jstrings. Two Java classes are served: NativeControlLib (runtime control) and
//! RustValidationBridge (config/identity validation).

use std::ffi::CString;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};

use crate::c_abi::*;

use super::{AndroidTunnelController, last_error_for_handle, to_jstring};
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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_phillipchin_webrtctunnel_NativeControlLib_nativeLastError(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jstring {
    let error = last_error_for_handle(handle as *mut AndroidTunnelController);
    to_jstring(&mut env, error)
}
