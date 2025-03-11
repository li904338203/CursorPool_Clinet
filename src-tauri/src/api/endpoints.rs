use super::client::get_base_url;
use super::types::*;
use tauri::State;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize)]
pub struct BugReportRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub app_version: String,
    pub os_version: String,
    pub device_model: String,
    pub cursor_version: String,
    pub bug_description: String,
    pub occurrence_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_urls: Option<Vec<String>>,
    pub severity: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterRequest {
    pub tenantId: String,
    pub account: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BladeResponse<T> {
    pub code: i32,
    pub success: bool,
    pub data: T,
    pub msg: String,
}

#[tauri::command]
pub async fn check_user(
    client: State<'_, super::client::ApiClient>,
    username: String,
) -> Result<ApiResponse<CheckUserResponse>, String> {
    let response = client
        .0
        .post(format!("{}/user/check", get_base_url()))
        .json(&CheckUserRequest { username })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_code(
    client: State<'_, super::client::ApiClient>,
    username: String,
    is_reset_password: Option<bool>,
) -> Result<ApiResponse<SendCodeResponse>, String> {
    let response = client
        .0
        .post(format!("{}/user/send_code", get_base_url()))
        .json(&SendCodeRequest {
            username,
            is_reset_password,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 先获取响应文本
    let response_text = response.text().await.map_err(|e| e.to_string())?;
    // 打印响应文本用于调试
    println!("Send code response: {}", response_text);
    // 解析JSON响应
    serde_json::from_str(&response_text).map_err(|e| e.to_string())
    // response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn login(
    client: State<'_, super::client::ApiClient>,
    username: String,
    password: String,
    device_id: String,
    tenant_id: Option<String>,
    sms_code: Option<String>,
) -> Result<LoginResponse, String> {
    println!("登录请求: username={}, device_id={}, tenant_id={:?}", username, device_id, tenant_id);
    
    // 如果提供了tenant_id，直接使用；否则查找用户的tenantId
    let tenant_id = if let Some(id) = tenant_id {
        id
    } else {
        // 查找用户的tenantId
        let tenant_id_response = client
            .0
            .get(format!("http://27.25.153.228:8080/api/blade-system/user/getTenantId?account={}", username))
            .header("Content-Type", "application/json")
            .header("Authorization", "Basic c2FiZXI6c2FiZXJfc2VjcmV0")
            .send()
            .await
            .map_err(|e| {
                println!("查找用户失败: {}", e);
                e.to_string()
            })?;
        
        println!("查找用户响应状态码: {}", tenant_id_response.status());
        
        // 如果状态码是400，表示用户不存在
        if tenant_id_response.status() == 400 {
            println!("用户不存在");
            return Err("用户不存在".to_string());
        }
        
        let tenant_id_text = tenant_id_response.text().await.map_err(|e| {
            println!("读取查找用户响应失败: {}", e);
            e.to_string()
        })?;
        
        println!("查找用户响应内容: {}", tenant_id_text);
        
        // 解析为BladeResponse格式
        let blade_response: BladeResponse<serde_json::Value> = serde_json::from_str(&tenant_id_text).map_err(|e| {
            println!("解析查找用户响应失败: {}", e);
            e.to_string()
        })?;
        
        // 检查是否成功
        if !blade_response.success || blade_response.code != 200 {
            println!("查找用户失败: {}", blade_response.msg);
            return Err(blade_response.msg);
        }
        
        // 获取tenantId
        match blade_response.data.as_str() {
            Some(id) => id.to_string(),
            None => {
                println!("无法获取tenantId");
                return Err("无法获取tenantId".to_string());
            }
        }
    };
    
    println!("使用tenantId={}", tenant_id);
    
    // 然后调用登录接口
    let login_url = format!(
        "http://27.25.153.228:8083/blade-auth/token?tenantId={}&account={}&password={}&type=password",
        tenant_id, username, password
    );
    
    println!("登录URL: {}", login_url);
    println!("Authorization: Basic c2FiZXI6c2FiZXJfc2VjcmV0");
    
    // 构建请求
    let request = client
        .0
        .post(login_url.clone())
        .header("Content-Type", "application/json")
        .header("Authorization", "Basic c2FiZXI6c2FiZXJfc2VjcmV0");
    
    // 打印请求信息
    println!("请求方法: POST");
    println!("请求URL: {}", login_url);
    println!("请求头: Content-Type: application/json");
    println!("请求头: Authorization: Basic c2FiZXI6c2FiZXJfc2VjcmV0");
    
    // 发送请求
    let login_response = request
        .send()
        .await
        .map_err(|e| {
            println!("登录请求失败: {}", e);
            e.to_string()
        })?;
    
    println!("登录响应状态码: {}", login_response.status());
    
    let login_text = login_response.text().await.map_err(|e| {
        println!("读取登录响应失败: {}", e);
        e.to_string()
    })?;
    
    println!("登录响应内容: {}", login_text);
    
    // 解析为BladeResponse格式
    let login_blade_response: BladeResponse<serde_json::Value> = serde_json::from_str(&login_text).map_err(|e| {
        println!("解析登录响应失败: {}", e);
        e.to_string()
    })?;
    
    // 检查是否成功
    if !login_blade_response.success || login_blade_response.code != 200 {
        println!("登录失败: {}", login_blade_response.msg);
        return Err(login_blade_response.msg);
    }
    
    // 获取token
    let token_data = match login_blade_response.data.as_object() {
        Some(obj) => obj,
        None => {
            println!("无法获取token数据");
            return Err("无法获取token数据".to_string());
        }
    };
    
    // 兼容新的返回数据格式，尝试获取accessToken或access_token
    let token = match token_data.get("accessToken").and_then(|t| t.as_str()) {
        Some(t) => t.to_string(),
        None => match token_data.get("access_token").and_then(|t| t.as_str()) {
            Some(t) => t.to_string(),
            None => {
                println!("无法获取token");
                return Err("无法获取token".to_string());
            }
        }
    };
    
    println!("登录成功, token={}", token);
    
    Ok(LoginResponse {
        api_key: Some(token),
    })
}

#[tauri::command]
pub async fn get_user_info(
    client: State<'_, super::client::ApiClient>,
    api_key: String,
) -> Result<ApiResponse<UserInfo>, String> {
    let response = client
        .0
        .get(format!("{}/user/info", get_base_url()))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn activate(
    client: State<'_, super::client::ApiClient>,
    token: String,
    code: String,
) -> Result<ApiResponse<ActivateResponse>, String> {
    // 构造新的请求体
    let request_body = serde_json::json!({
        "cardKey": code
    });

    // 发送请求到新的接口地址
    let response = client
        .0
        .post("http://27.25.153.228:8080/api/blade-system/cardKey/useNew")
        .header("Blade-Auth", format!("bearer {}", token))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 解析响应
    let new_response: NewApiResponse<bool> = response.json().await.map_err(|e| e.to_string())?;
    
    // 如果请求成功，返回一个兼容原有格式的响应
    if new_response.success && new_response.code == 200 {
        // 返回一个兼容原有格式的响应
        Ok(ApiResponse {
            status: "success".to_string(),
            message: new_response.msg,
            data: Some(ActivateResponse {
                expire_time: (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() * 1000, // 默认30天过期
                level: 1, // 默认等级
            }),
        })
    } else {
        // 如果请求失败，返回错误信息
        Err(new_response.msg)
    }
}

#[tauri::command]
pub async fn change_password(
    client: State<'_, super::client::ApiClient>,
    api_key: String,
    old_password: String,
    new_password: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    let response = client
        .0
        .post(format!("{}/user/change_password", get_base_url()))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&ChangePasswordRequest {
            old_password,
            new_password,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 先打印原始响应内容
    let raw_response = response.text().await.map_err(|e| e.to_string())?;
    println!("Raw change password response: {}", raw_response);

    // 尝试解析为JSON
    let result: ApiResponse<LoginResponse> = serde_json::from_str(&raw_response)
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    Ok(result)
}

#[tauri::command]
pub async fn get_account(
    client: State<'_, super::client::ApiClient>,
    api_key: String,
) -> Result<ApiResponse<AccountDetail>, String> {
    let response = client
        .0
        .get(format!("{}/account/get", get_base_url()))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let account_response: ApiResponse<AccountInfo> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    // 只返回需要的字段
    Ok(ApiResponse {
        status: account_response.status,
        message: account_response.message,
        data: account_response.data.map(|account_info| {
            let parts: Vec<&str> = account_info.token.split("%3A%3A").collect();
            AccountDetail {
                email: account_info.email,
                user_id: parts[0].to_string(),
                token: parts[1].to_string(),
            }
        }),
    })
}

#[tauri::command]
pub async fn get_usage(
    client: State<'_, super::client::ApiClient>,
    token: String,
) -> Result<ApiResponse<CursorUsageInfo>, String> {
    let user_id = "user_01000000000000000000000000";
    let response = client
        .0
        .get("https://www.cursor.com/api/usage")
        .header("Cookie", format!("WorkosCursorSessionToken={}%3A%3A{}", user_id, token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let usage_info: CursorUsageInfo = response.json().await.map_err(|e| e.to_string())?;
    Ok(ApiResponse {
        status: "success".to_string(),
        message: "获取使用情况成功".to_string(),
        data: Some(usage_info),
    })
}

#[tauri::command]
pub async fn get_user_info_cursor(
    client: State<'_, super::client::ApiClient>,
    user_id: String,
    token: String,
) -> Result<ApiResponse<CursorUserInfo>, String> {
    let response = client
        .0
        .get("https://www.cursor.com/api/auth/me")
        .header("Cookie", format!("WorkosCursorSessionToken={}%3A%3A{}", user_id, token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let user_info: CursorUserInfo = response.json().await.map_err(|e| e.to_string())?;
    Ok(ApiResponse {
        status: "success".to_string(),
        message: "获取用户信息成功".to_string(),
        data: Some(user_info),
    })
}

#[tauri::command]
pub async fn get_version(
    client: State<'_, super::client::ApiClient>,
) -> Result<ApiResponse<VersionInfo>, String> {
    let response = client
        .0
        .get(format!("{}/version", get_base_url()))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_str(&response.text().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_public_info(
    client: State<'_, super::client::ApiClient>,
) -> Result<ApiResponse<PublicInfo>, String> {
    let response = client
        .0
        .get(format!("{}/public/info", get_base_url()))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_password(
    client: State<'_, super::client::ApiClient>,
    email: String,
    sms_code: String,
    new_password: String,
) -> Result<ApiResponse<()>, String> {
    let response = client
        .0
        .post(format!("{}/user/reset_password", get_base_url()))
        .json(&ResetPasswordRequest {
            email,
            sms_code,
            new_password,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_bug(
    client: State<'_, super::client::ApiClient>,
    severity: String,
    bug_description: String,
    api_key: Option<String>,
    screenshot_urls: Option<Vec<String>>,
    cursor_version: Option<String>,
) -> Result<(), String> {
    // 获取应用版本
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    
    // 获取操作系统信息
    let os_info = os_info::get();
    let os_version = format!("{} {}", os_info.os_type(), os_info.version());
    
    // 获取设备型号
    let device_model = sys_info::hostname()
        .unwrap_or_else(|_| "Unknown".to_string());
    
    // 获取当前时间，ISO 8601 格式
    let occurrence_time = Utc::now().to_rfc3339();
    
    // 获取 Cursor 版本，如果未提供则从数据库获取
    let cursor_version = cursor_version.unwrap_or_else(|| {
        crate::utils::CursorVersion::get_version()
            .unwrap_or_else(|_| "Unknown".to_string())
    });
    
    // 创建请求体
    let report = BugReportRequest {
        api_key,
        app_version,
        os_version,
        device_model,
        cursor_version,
        bug_description,
        occurrence_time,
        screenshot_urls,
        severity,
    };

    // 发送请求
    let _response = client
        .0
        .post(format!("{}/report", get_base_url()))
        .json(&report)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_disclaimer(
    client: State<'_, super::client::ApiClient>,
) -> Result<ApiResponse<DisclaimerResponse>, String> {
    let response = client
        .0
        .get(format!("{}/disclaimer", get_base_url()))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register(
    client: State<'_, super::client::ApiClient>,
    tenantId: String,
    account: String,
    password: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    println!("后端收到注册请求: tenantId={}, account={}, password={}", tenantId, account, password);
    
    let request = RegisterRequest {
        tenantId,
        account,
        password,
    };
    
    let request_json = serde_json::to_string(&request).unwrap_or_default();
    println!("构建的请求体: {}", request_json);
    
    let response = client
        .0
        .post(format!("http://27.25.153.228:8080/api/blade-system/user/register"))
        .header("Content-Type", "application/json")
        .header("Authorization", "Basic c2FiZXI6c2FiZXJfc2VjcmV0")
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            println!("发送请求失败: {}", e);
            e.to_string()
        })?;
    
    println!("收到响应状态码: {}", response.status());
    
    let response_text = response.text().await.map_err(|e| {
        println!("读取响应文本失败: {}", e);
        e.to_string()
    })?;
    
    println!("响应内容: {}", response_text);
    
    // 解析为BladeResponse格式
    let blade_response: BladeResponse<serde_json::Value> = serde_json::from_str(&response_text).map_err(|e| {
        println!("解析响应失败: {}", e);
        e.to_string()
    })?;
    
    println!("解析响应成功: code={}, success={}, msg={}", blade_response.code, blade_response.success, blade_response.msg);
    
    // 检查是否成功
    if !blade_response.success || blade_response.code != 200 {
        println!("注册失败: {}", blade_response.msg);
        return Err(blade_response.msg);
    }
    
    // 构造一个ApiResponse
    let api_response = ApiResponse {
        status: "success".to_string(),
        message: blade_response.msg,
        data: Some(LoginResponse {
            api_key: Some("dummy_api_key".to_string()),
        }),
    };
    
    println!("注册成功");
    Ok(api_response)
}

#[tauri::command]
pub async fn get_tenant_id(
    client: State<'_, super::client::ApiClient>,
    account: String,
) -> Result<ApiResponse<String>, String> {
    println!("查找用户: account={}", account);
    
    let response = client
        .0
        .get(format!("http://27.25.153.228:8080/api/blade-system/user/getTenantId?account={}", account))
        .header("Content-Type", "application/json")
        .header("Authorization", "Basic c2FiZXI6c2FiZXJfc2VjcmV0")
        .send()
        .await
        .map_err(|e| {
            println!("发送请求失败: {}", e);
            e.to_string()
        })?;
    
    println!("收到响应状态码: {}", response.status());
    
    let response_text = response.text().await.map_err(|e| {
        println!("读取响应文本失败: {}", e);
        e.to_string()
    })?;
    
    println!("响应内容: {}", response_text);
    
    // 解析为BladeResponse格式
    let blade_response: BladeResponse<serde_json::Value> = serde_json::from_str(&response_text).map_err(|e| {
        println!("解析响应失败: {}", e);
        e.to_string()
    })?;
    
    println!("解析响应成功: code={}, success={}, msg={}", blade_response.code, blade_response.success, blade_response.msg);
    
    // 检查是否成功
    if !blade_response.success || blade_response.code != 200 {
        println!("查找用户失败: {}", blade_response.msg);
        return Err(blade_response.msg);
    }
    
    // 获取tenantId
    let tenant_id = match blade_response.data.as_str() {
        Some(id) => id.to_string(),
        None => {
            println!("无法获取tenantId");
            return Err("无法获取tenantId".to_string());
        }
    };
    
    println!("查找用户成功, tenantId={}", tenant_id);
    
    // 构造一个ApiResponse
    let api_response = ApiResponse {
        status: "success".to_string(),
        message: blade_response.msg,
        data: Some(tenant_id),
    };
    
    Ok(api_response)
}
