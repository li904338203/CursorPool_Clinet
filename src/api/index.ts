import { invoke } from '@tauri-apps/api/core'
import type {
    ApiResponse,
    LoginRequest,
    LoginResponse,
    UserInfo,
    CheckUserResponse,
    SendCodeResponse,
    AccountDetail,
    UsageInfo,
    UserInfoResponse,
    VersionInfo,
    PublicInfo,
    MachineInfo,
    ActivateResponse,
    DisclaimerResponse,
    RegisterRequest,
    UserDetailResponse
} from './types'

// 错误处理
function handleApiResponse<T>(response: ApiResponse<T>): T {
    if (response.status === 'success') {
        // 成功时返回 data 或 message
        return response.data || response.message as unknown as T
    }
    throw new Error(response.message || 'API request failed')
}

// API 错误类
export class ApiError extends Error {
    constructor(message: string) {
        super(message)
        this.name = 'ApiError'
    }
}

// 用户认证相关 API
export async function checkUser(username: string): Promise<CheckUserResponse> {
    try {
        const response = await invoke<ApiResponse<CheckUserResponse>>('check_user', { username })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check user')
    }
}

export async function sendCode(username: string, isResetPassword?: boolean): Promise<SendCodeResponse> {
    try {
        const response = await invoke<ApiResponse<SendCodeResponse>>('send_code', { username, isResetPassword })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to send code')
    }
}

export async function login(params: LoginRequest): Promise<LoginResponse> {
    try {
        console.log('登录请求参数:', params);
        
        // 如果没有tenantId，先获取
        let tenantId = params.tenantId;
        if (!tenantId) {
            tenantId = await getTenantId(params.username);
        }
        
        // 构建登录URL
        const loginUrl = `http://27.25.153.228:8083/blade-auth/token?tenantId=${tenantId}&account=${params.username}&password=${params.password}&type=password`;
        
        console.log('登录URL:', loginUrl);
        
        // 使用POST方法调用登录接口
        const response = await fetch(loginUrl, {
            method: 'POST', // 改为POST方法
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Basic c2FiZXI6c2FiZXJfc2VjcmV0'
            }
        });
        
        if (!response.ok) {
            throw new Error(`登录失败: ${response.status} ${response.statusText}`);
        }
        
        const data = await response.json();
        console.log('登录响应:', data);
        
        if (!data.success || data.code !== 200) {
            throw new Error(data.msg || '登录失败');
        }
        
        // 从响应中提取token，兼容新的返回数据格式
        const token = data.data?.accessToken || data.data?.access_token;
        if (!token) {
            throw new Error('无法获取token');
        }
        
        // 保存完整的用户数据到localStorage
        localStorage.setItem('userInfo', JSON.stringify(data.data));
        
        // 保存token到localStorage
        localStorage.setItem('accessToken', token);
        
        // 返回兼容原来接口的数据格式
        return { accessToken: token };
    } catch (error) {
        console.error('登录失败:', error);
        throw new ApiError(error instanceof Error ? error.message : 'Failed to login');
    }
}

export async function register(params: RegisterRequest): Promise<boolean> {
    try {
        console.log('发送注册请求:', params); // 添加日志
        
        // 直接使用params对象，不做字段映射
        const response = await invoke<ApiResponse<any>>('register', {
            tenantId: params.tenantId,
            account: params.account,
            password: params.password
        })
        
        console.log('注册响应:', response);
        
        // 如果响应成功，返回true
        return response.status === 'success';
    } catch (error) {
        console.error('注册失败:', error);
        throw new ApiError(error instanceof Error ? error.message : 'Failed to register');
    }
}

// 用户信息相关 API
export async function getUserDetail(token: string): Promise<UserDetailResponse> {
    try {
        const response = await fetch('http://27.25.153.228:8083/blade-system/user/info', {
            headers: {
                'blade-auth': `Bearer ${token}`
            }
        });

        if (!response.ok) {
            throw new Error(`获取用户详情失败: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        if (!data.success || data.code !== 200) {
            throw new Error(data.msg || '获取用户详情失败');
        }

        return data;
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get user detail');
    }
}

export async function getUserInfo(token: string): Promise<UserInfo> {
    try {
        // 获取用户详细信息
        const userDetail = await getUserDetail(token);
        
        // 构造用户信息
        const userInfo: UserInfo = {
            totalCount: userDetail.data.balance + userDetail.data.bonus,
            usedCount: 0,     // 这里可能需要从其他接口获取已使用额度
            expireTime: Date.now() + 30 * 24 * 60 * 60 * 1000, // 默认30天
            level: 1,         // 默认等级
            isExpired: false,
            username: userDetail.data.realName || userDetail.data.account,
            email: '',
            credits: userDetail.data.balance + userDetail.data.bonus,
            balance: userDetail.data.balance,
            bonus: userDetail.data.bonus,
            usage: {
                'gpt-4': {
                    numRequests: 0
                },
                'gpt-3.5-turbo': {
                    numRequests: 0
                }
            }
        };
        
        // 保存用户数据到localStorage
        localStorage.setItem('userInfo', JSON.stringify(userInfo));
        
        return userInfo;
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get user info');
    }
}

export async function getAccount(token: string): Promise<AccountDetail> {
    try {
        const response = await invoke<ApiResponse<AccountDetail>>('get_account', { token })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get account info')
    }
}

// Cursor 平台相关 API
export async function getUserInfoCursor(token: string): Promise<UserInfoResponse> {
    try {
        const response = await invoke<ApiResponse<UserInfoResponse>>('get_user_info_cursor', { token })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get cursor user info')
    }
}

export async function getUsage(token: string): Promise<UsageInfo> {
    try {
        const response = await invoke<ApiResponse<UsageInfo>>('get_usage', { token })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get usage info')
    }
}

// 系统信息相关 API
export async function getPublicInfo(): Promise<PublicInfo> {
    try {
        const response = await invoke<ApiResponse<PublicInfo>>('get_public_info')
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get public info')
    }
}

export async function getVersion(): Promise<VersionInfo> {
    try {
        const response = await invoke<ApiResponse<VersionInfo>>('get_version')
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get version info')
    }
}

// 账户管理相关 API
export async function activate(token: string, code: string): Promise<ActivateResponse> {
    try {
        const response = await invoke<ApiResponse<ActivateResponse>>('activate', { token, code })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to activate')
    }
}

export async function changePassword(token: string, old_password: string, new_password: string): Promise<boolean> {
    try {
        // 使用fetch直接调用API
        const response = await fetch('https://cursor.92xx.vip/blade-system/user/update-password?oldPassword=' + 
            encodeURIComponent(old_password) + 
            '&newPassword=' + encodeURIComponent(new_password) + 
            '&newPassword1=' + encodeURIComponent(new_password), {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Blade-Auth': 'Bearer ' + token
            }
        });
        
        const data = await response.json();
        
        if (data.code === 200 && data.success) {
            return true;
        } else {
            throw new Error(data.msg || '修改密码失败');
        }
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : '修改密码失败')
    }
}

// 机器码和账户切换相关 API
export async function resetMachineId(params: { forceKill?: boolean, machineId?: string } = {}): Promise<boolean> {
    try {
        return await invoke<boolean>('reset_machine_id', { 
            forceKill: params.forceKill || false,
            machineId: params.machineId
        })
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to reset machine id')
    }
}

export async function switchAccount(email: string, token: string, force_kill: boolean = false): Promise<void> {
    try {
        const result = await invoke<boolean>('switch_account', { email, token, forceKill: force_kill })
        if (result !== true) {
            throw new Error('切换账户失败')
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to switch account'
        if (errorMsg.includes('Cursor进程正在运行, 请先关闭Cursor')) {
            throw new Error('请先关闭 Cursor 或选择强制终止进程')
        }
        throw error
    }
}

export async function getMachineIds(): Promise<MachineInfo> {
    try {
        return await invoke<MachineInfo>('get_machine_ids')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get machine IDs')
    }
}

export async function checkCursorRunning(): Promise<boolean> {
    try {
        return await invoke<boolean>('check_cursor_running')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check cursor status')
    }
}

// 添加新的 kill_cursor_process API
export async function killCursorProcess(): Promise<void> {
    try {
        await invoke<void>('kill_cursor_process')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to kill cursor process')
    }
}

// 添加 waitForCursorClose 辅助函数
export async function waitForCursorClose(timeout = 10000): Promise<boolean> {
    const startTime = Date.now()
    
    while (Date.now() - startTime < timeout) {
        const isRunning = await checkCursorRunning()
        if (!isRunning) {
            return true
        }
        await new Promise(resolve => setTimeout(resolve, 500))
    }
    
    throw new ApiError('关闭 Cursor 超时')
}

// 管理员权限相关 API
export async function checkAdminPrivileges(): Promise<boolean> {
    try {
        return await invoke<boolean>('check_admin_privileges')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check admin privileges')
    }
}

// Cursor 更新控制相关 API
export async function disableCursorUpdate(force_kill: boolean = false): Promise<void> {
    try {
        await invoke<void>('disable_cursor_update', { forceKill: force_kill })
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to disable cursor update'
        if (errorMsg.includes('Cursor进程正在运行')) {
            throw new Error('请先关闭 Cursor 或选择强制终止进程')
        }
        throw error
    }
}

export async function restoreCursorUpdate(force_kill: boolean = false): Promise<void> {
    try {
        await invoke<void>('restore_cursor_update', { forceKill: force_kill })
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to restore cursor update'
        if (errorMsg.includes('Cursor进程正在运行')) {
            throw new Error('请先关闭 Cursor 或选择强制终止进程')
        }
        throw error
    }
}

export async function checkUpdateDisabled(): Promise<boolean> {
    try {
        return await invoke<boolean>('check_update_disabled')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check update status')
    }
}

// Hook 相关 API
export async function checkHookStatus(): Promise<boolean> {
    try {
        return await invoke<boolean>('is_hook')
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check hook status')
    }
}

export async function applyHook(force_kill: boolean = false): Promise<void> {
    try {
        await invoke<void>('hook_main_js', { forceKill: force_kill })
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to apply hook'
        if (errorMsg.includes('Cursor进程正在运行')) {
            throw new Error('请先关闭 Cursor 或选择强制终止进程')
        }
        throw error
    }
}

export async function restoreHook(force_kill: boolean = false): Promise<void> {
    try {
        await invoke<void>('restore_hook', { forceKill: force_kill })
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to restore hook'
        if (errorMsg.includes('Cursor进程正在运行')) {
            throw new Error('请先关闭 Cursor 或选择强制终止进程')
        }
        throw error
    }
}

export async function resetPassword(email: string, smsCode: string, newPassword: string): Promise<string> {
    try {
        const response = await invoke<ApiResponse<string>>('reset_password', { 
            email, 
            smsCode, 
            newPassword 
        })
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to reset password')
    }
}

// 添加新的 API 函数来检测系统是否为 Windows
export async function checkIsWindows(): Promise<boolean> {
    try {
        return await invoke<boolean>('check_is_windows');
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to check if system is Windows');
    }
}

// 获取免责声明
export async function getDisclaimer(): Promise<DisclaimerResponse> {
    try {
        const response = await invoke<ApiResponse<DisclaimerResponse>>('get_disclaimer')
        return handleApiResponse(response)
    } catch (error) {
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get disclaimer')
    }
}

// 添加关闭和启动Cursor的API
export async function closeCursor(): Promise<boolean> {
  return await invoke('close_cursor')
}

export async function launchCursor(): Promise<boolean> {
  return await invoke('launch_cursor')
}

export async function getTenantId(account: string): Promise<string> {
    try {
        console.log('查找用户:', account);
        
        const response = await invoke<ApiResponse<string>>('get_tenant_id', { account });
        
        console.log('查找用户响应:', response);
        
        if (response.status === 'success' && response.data) {
            return response.data;
        }
        
        throw new Error(response.message || '查找用户失败');
    } catch (error) {
        console.error('查找用户失败:', error);
        throw new ApiError(error instanceof Error ? error.message : 'Failed to get tenant ID');
    }
}