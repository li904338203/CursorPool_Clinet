// 通用响应类型
export interface ApiResponse<T> {
  status: string
  message: string
  data?: T
}

// 用户信息
export interface UserInfo {
  totalCount: number
  usedCount: number
  expireTime: number
  level: number
  isExpired: boolean
  username: string
  email: string
  credits: number
  balance: number
  bonus: number
  usage: {
    'gpt-4': {
      numRequests: number
    }
    'gpt-3.5-turbo': {
      numRequests: number
    }
  }
}

// 账户信息
export interface AccountInfo {
  email: string
  token: string
  usedCount: number
  totalLimit: number
}

// 账户详细信息
export interface AccountDetail {
  email: string
  userId: string
  token: string
}

// 登录请求
export interface LoginRequest {
  username: string
  password: string
  deviceId: string
  tenantId?: string
  smsCode?: string
}

// 注册请求
export interface RegisterRequest {
  tenantId: string
  account: string
  password: string
}

// 登录响应
export interface LoginResponse {
  accessToken?: string
}

// 检查用户请求
export interface CheckUserRequest {
  username: string
}

// 检查用户响应
export interface CheckUserResponse {
  exists: boolean
  needCode: boolean
}

// 发送验证码请求
export interface SendCodeRequest {
  username: string
  isResetPassword?: boolean
}

// 发送验证码响应
export interface SendCodeResponse {
  expireIn: number
}

// 激活请求
export interface ActivateRequest {
  code: string
}

// 激活响应
export interface ActivateResponse {
  expireTime: number
  level: number
}

// 修改密码请求
export interface ChangePasswordRequest {
  oldPassword: string
  newPassword: string
}

// 版本信息响应
export interface VersionInfo {
  version: string
  forceUpdate: boolean
  downloadUrl: string
  changeLog: string
}

// 公告信息响应
export interface PublicInfo {
  type: string
  closeable: boolean
  props: PublicInfoProps
  actions: PublicInfoAction[]
}

export interface PublicInfoProps {
  title: string
  description: string
}

export interface PublicInfoAction {
  type: string
  text: string
  url: string
}

// GPT 模型使用情况
export interface GptModelUsage {
  numRequests: number
  numRequestsTotal: number
  numTokens: number
  maxRequestUsage?: number
  maxTokenUsage?: number
}

// 使用情况响应
export interface UsageInfo extends CursorUsageInfo {}

// 用户信息响应（Cursor平台）
export interface UserInfoResponse extends CursorUserInfo {}

// Cursor 用户信息
export interface CursorUserInfo {
  email: string
  email_verified: boolean
  name: string
  sub: string
  updatedAt: string
  picture: string | null
}

// Cursor 模型使用情况
export interface CursorModelUsage {
  numRequests: number
  numRequestsTotal: number
  numTokens: number
  maxRequestUsage: number | null
  maxTokenUsage: number | null
}

// Cursor 使用情况
export interface CursorUsageInfo {
  'gpt-4': CursorModelUsage
  'gpt-3.5-turbo': CursorModelUsage
  'gpt-4-32k': CursorModelUsage
  startOfMonth: string
}

// 设备信息
export interface DeviceInfo {
  machineCode: string
  currentAccount: string
  userInfo: UserInfo | null
  cursorInfo: {
    userInfo: CursorUserInfo | null
    usage: CursorUsageInfo | null
  } | null
}

// 机器码信息
export interface MachineInfo {
  machineId: string
  currentAccount: string
  cursorToken: string
  machineCode: string
}

// 管理员权限检查响应
export interface AdminPrivilegesInfo {
  isAdmin: boolean
}

// Cursor 更新状态
export interface UpdateStatus {
  isDisabled: boolean
}

// Hook 状态信息
export interface HookStatus {
  isHooked: boolean
}

// Hook 操作响应
export interface HookResponse {
  success: boolean
}

// 重置密码请求
export interface ResetPasswordRequest {
  email: string
  smsCode: string
  newPassword: string
}

// 系统信息
export interface SystemInfo {
    isWindows: boolean;
}

// 免责声明响应
export interface DisclaimerResponse {
  content: string
}

// 重置机器码请求
export interface ResetMachineIdRequest {
  forceKill: boolean
  machineId?: string  // 可选参数
}

export interface GetTenantIdResponse {
  tenantId: string
}

export interface UserDetailResponse {
  code: number
  success: boolean
  data: {
    id: string
    account: string
    realName: string
    balance: number
    bonus: number
    cursorAccountsId: number
    roleName: string
    deptName: string
  }
  msg: string
}