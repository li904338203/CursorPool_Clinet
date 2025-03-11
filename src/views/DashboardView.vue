<script setup lang="ts">
import { NCard, NSpace, NButton, NProgress, NNumberAnimation, NGrid, NGridItem, NTag, NDivider, NModal, NIcon, NForm, NFormItem, NInput } from 'naive-ui'
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from '../locales'
import { useMessage } from 'naive-ui'
import { 
    getUserInfo, 
    resetMachineId, 
    switchAccount, 
    getMachineIds, 
    getUsage, 
    getAccount, 
    getVersion, 
    checkCursorRunning,
    checkAdminPrivileges,
    checkUpdateDisabled,
    checkHookStatus,
    checkIsWindows,
    applyHook,
    activate,
    closeCursor,
    launchCursor
} from '@/api'
import type { UserInfo, CursorUserInfo, CursorUsageInfo, VersionInfo } from '@/api/types'
import { addHistoryRecord } from '../utils/history'
import { version } from '../../package.json'
import { WarningOutlined } from '@vicons/antd'
import { Window } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-shell'
import { saveAccountToHistory } from '@/utils/historyAccounts'
import type { HistoryAccount } from '@/types/history'
import DashboardTour from '../components/DashboardTour.vue'

const LOCAL_VERSION = version

// 版本检查的时间间隔（毫秒）
const VERSION_CHECK_INTERVAL = 3 * 60 * 60 * 1000 // 3小时

interface DeviceInfoState {
  machineCode: string
  currentAccount: string
  cursorToken: string
  userInfo: UserInfo | null
  cursorInfo: {
    userInfo: CursorUserInfo | null
    usage: CursorUsageInfo | null
  }
  hookStatus: boolean | null
}

// 获取当前时间段的问候语
function getTimeOfDay() {
  const hour = new Date().getHours()
  if (hour < 6) return '夜深了'
  if (hour < 12) return '早上好'
  if (hour < 18) return '下午好'
  return '晚上好'
}

// 格式化日期
const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

const deviceInfo = ref<DeviceInfoState>({
  machineCode: '',
  currentAccount: '',
  cursorToken: '',
  userInfo: null,
  cursorInfo: {
    userInfo: null,
    usage: null
  },
  hookStatus: null
})

const loading = ref(true)

const message = useMessage()
const { i18n } = useI18n()

// 计算使用量百分比
const getUsagePercentage = (used: number, total: number) => {
  if (!total) return 0
  return Math.min(100, Math.round((used / total) * 100))
}

// 会员等级映射
const levelMap: Record<number, { name: string; type: 'default' | 'info' | 'success' | 'warning' | 'error' | 'primary' }> = {
  1: { name: i18n.value.dashboard.memberLevel[1], type: 'primary' },
  2: { name: i18n.value.dashboard.memberLevel[2], type: 'primary' },
  3: { name: i18n.value.dashboard.memberLevel[3], type: 'primary' },
  4: { name: i18n.value.dashboard.memberLevel[4], type: 'primary' },
  5: { name: i18n.value.dashboard.memberLevel[5], type: 'primary' }
}

// Cursor高级模型使用量百分比
const cursorGpt4Percentage = computed(() => {
  const usage = deviceInfo.value.cursorInfo.usage?.['gpt-4']
  if (!usage) return 0
  return getUsagePercentage(usage.numRequests, usage.maxRequestUsage || 0)
})

// Cursor普通模型使用量百分比
const cursorGpt35Percentage = computed(() => {
  const usage = deviceInfo.value.cursorInfo.usage?.['gpt-3.5-turbo']
  if (!usage) return 0
  if (!usage.maxRequestUsage) return 100
  return getUsagePercentage(usage.numRequests, usage.maxRequestUsage)
})

// 获取用户信息
const fetchUserInfo = async () => {
  try {
    // 只使用accessToken
    const accessToken = localStorage.getItem('accessToken');
    
    if (!accessToken) {
      throw new Error('未找到 Token');
    }
    
    // 使用accessToken
    const info = await getUserInfo(accessToken);
    deviceInfo.value.userInfo = info;
  } catch (error) {
    // 如果获取失败，清除token
    localStorage.removeItem('accessToken');
    console.error('获取用户信息失败:', error);
  }
}

// 获取机器码
const fetchMachineIds = async () => {
  try {
    const result = await getMachineIds()

    deviceInfo.value.machineCode = result.machineId
    deviceInfo.value.currentAccount = result.currentAccount
    deviceInfo.value.cursorToken = result.cursorToken
    
    // 获取 Hook 状态
    deviceInfo.value.hookStatus = await checkHookStatus()
  } catch (error) {
    console.error('获取机器码失败:', error)
  }
}

// 获取 Cursor 账户信息
async function fetchCursorInfo() {
  try {
    const token = deviceInfo.value.cursorToken
    if (!token) {
      console.error('未找到 Cursor Token')
      return
    }

    const usageData = await getUsage(token)
    
    deviceInfo.value.cursorInfo = {
      userInfo: {
        email: deviceInfo.value.currentAccount,
        email_verified: true,
        name: deviceInfo.value.currentAccount.split('@')[0],
        sub: '',
        updatedAt: new Date().toISOString(),
        picture: null
      },
      usage: usageData
    }
  } catch (error) {
    console.error('获取 Cursor 账户信息失败:', error)
  } finally {
    loading.value = false
  }
}

// 添加新的 ref
const showCursorRunningModal = ref(false)
const pendingForceKillAction = ref<{
  type: 'machine' | 'account' | 'quick' | 'hook',
  params?: any
} | null>(null)

// 修改机器码更换处理函数
const handleMachineCodeChange = async (force_kill: boolean = false) => {
  try {
    await resetMachineId({ forceKill: force_kill })
    message.success(i18n.value.dashboard.machineChangeSuccess)
    addHistoryRecord(
      '机器码修改',
      `修改机器码: ${deviceInfo.value.machineCode}`
    )
    await fetchMachineIds()
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    if (errorMsg === 'Cursor进程正在运行, 请先关闭Cursor') {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'machine' }
      return
    }
    message.error(i18n.value.dashboard.machineChangeFailed)
  }
}

// 添加新的 ref
const showUnusedCreditsModal = ref(false)
const unusedCredits = ref(0)
const pendingAction = ref<'account' | 'quick' | null>(null)

// 添加统一的自动注入函数，并增加日志
const autoApplyHook = async (): Promise<boolean> => {
  try {
    message.loading('正在自动注入...', { duration: 0 })
    await applyHook(false)
    message.destroyAll()
    message.success(i18n.value.systemControl.messages.applyHookSuccess)
    
    // 更新Hook状态
    deviceInfo.value.hookStatus = await checkHookStatus()

    // 返回注入结果
    return deviceInfo.value.hookStatus === true
  } catch (error) {
    console.error('自动注入失败:', error)
    message.destroyAll()
    message.error(error instanceof Error ? error.message : '注入失败，请手动注入后再试')
    return false
  }
}

// 修改账户切换处理函数
const handleAccountSwitch = async () => {
  try {
    accountSwitchLoading.value = true
    
    // 保存当前账户到历史记录
    if (deviceInfo.value.cursorInfo?.userInfo) {
      const historyAccount: HistoryAccount = {
        email: deviceInfo.value.cursorInfo.userInfo.email,
        token: deviceInfo.value.cursorToken,
        machineCode: deviceInfo.value.machineCode,
        gpt4Count: deviceInfo.value.cursorInfo.usage?.['gpt-4']?.numRequests || 0,
        gpt35Count: deviceInfo.value.cursorInfo.usage?.['gpt-3.5-turbo']?.numRequests || 0,
        lastUsed: Date.now()
      }
      saveAccountToHistory(historyAccount)
    }
    
    // 检查积分是否足够
    if (userCredits.value < 50) {
      showInsufficientCreditsModal.value = true
      pendingCreditAction.value = 'account'
      return
    }
    
    // 检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    
    if (isRunning) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'account' }
      return
    }
    
    // 检查 CC 状态，如果未注入，直接调用注入
    if (!deviceInfo.value.hookStatus) {
      const hookSuccess = await autoApplyHook()
      
      if (hookSuccess) {
        await executeAccountSwitch()
      }
      return
    }

    await executeAccountSwitch()
  } catch (error) {
    console.error('账户切换失败:', error)
    message.error('操作失败: ' + (error instanceof Error ? error.message : String(error)))
  } finally {
    accountSwitchLoading.value = false
  }
}

// 修改一键切换处理函数
const handleQuickChange = async () => {
  try {
    quickChangeLoading.value = true
    
    // 保存当前账户到历史记录
    if (deviceInfo.value.cursorInfo?.userInfo) {
      const historyAccount: HistoryAccount = {
        email: deviceInfo.value.cursorInfo.userInfo.email,
        token: deviceInfo.value.cursorToken,
        machineCode: deviceInfo.value.machineCode,
        gpt4Count: deviceInfo.value.cursorInfo.usage?.['gpt-4']?.numRequests || 0,
        gpt35Count: deviceInfo.value.cursorInfo.usage?.['gpt-3.5-turbo']?.numRequests || 0,
        lastUsed: Date.now()
      }
      saveAccountToHistory(historyAccount)
    }
    
    // 检查积分是否足够
    if (userCredits.value < 50) {
      showInsufficientCreditsModal.value = true
      pendingCreditAction.value = 'quick'
      return
    }
    
    // 检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    
    if (isRunning) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'quick' }
      return
    }
    
    // 检查 CC 状态，如果未注入，直接调用注入
    if (!deviceInfo.value.hookStatus) {
      const hookSuccess = await autoApplyHook()
      
      if (hookSuccess) {
        await executeQuickChange()
      }
      return
    }

    await executeQuickChange()
  } catch (error) {
    console.error('一键切换失败:', error)
    message.error('操作失败: ' + (error instanceof Error ? error.message : String(error)))
  } finally {
    quickChangeLoading.value = false
  }
}

// 修改确认切换函数
const handleConfirmSwitch = async () => {
  showUnusedCreditsModal.value = false
  if (pendingAction.value === 'account') {
    // 检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    if (isRunning) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'account' }
      return
    }
    await executeAccountSwitch()
  } else if (pendingAction.value === 'quick') {
    // 检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    if (isRunning) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'quick' }
      return
    }
    await executeQuickChange()
  }
  pendingAction.value = null
}

const handleCancelSwitch = () => {
  showUnusedCreditsModal.value = false
  pendingAction.value = null
}

// 修改账户切换执行函数
const executeAccountSwitch = async (force_kill: boolean = false) => {
  try {
    const accessToken = localStorage.getItem('accessToken')
    if (!accessToken) {
      message.error(i18n.value.message.pleaseInputEmail)
      return
    }

    // 先检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    if (isRunning && !force_kill) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'account' }
      return
    }

    // 获取账号信息并执行实际的切换
    const accountInfo = await getAccount(accessToken)
    
    if (!accountInfo.email || !accountInfo.token) {
      message.error(i18n.value.dashboard.accountChangeFailed)
      return
    }
    
    await switchAccount(accountInfo.email, accountInfo.token, force_kill)
    message.success(i18n.value.dashboard.accountChangeSuccess)
    addHistoryRecord(
      '账户切换',
      `切换到账户: ${accountInfo.email} 扣除50积分`
    )
    await Promise.all([
      fetchUserInfo(),
      fetchMachineIds(),
      fetchCursorInfo()
    ])
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    if (errorMsg === 'Cursor进程正在运行, 请先关闭Cursor') {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'account' }
      return
    }
    console.error('切换账户失败:', error)
    message.error(i18n.value.common.copyFailed)
  }
}

// 修改一键切换执行函数
const executeQuickChange = async (force_kill: boolean = false) => {
  try {
    await executeAccountSwitch(force_kill)
    await handleMachineCodeChange(force_kill)
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    if (errorMsg === 'Cursor进程正在运行, 请先关闭Cursor') {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'quick' }
      return
    }
    message.error(i18n.value.common.copyFailed)
  }
}

// 修改强制关闭处理函数
const handleForceKill = async () => {
  showCursorRunningModal.value = false
  if (!pendingForceKillAction.value) return

  try {
    loading.value = true
    message.loading('正在关闭 Cursor...', { duration: 0 })
    
    // 关闭Cursor
    await closeCursor()
    
    // 等待一段时间确保进程完全关闭
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    message.destroyAll() // 清除 loading 消息
    
    // 根据类型执行相应操作
    let operationSuccess = false
    let operationMessage = ''
    
    switch (pendingForceKillAction.value.type) {
      case 'machine':
      case 'account':
      case 'quick':
        // 先检查是否需要注入
        if (!deviceInfo.value.hookStatus) {
          message.loading('正在注入...', { duration: 0 })
          const hookSuccess = await autoApplyHook()
          
          if (!hookSuccess) {
            message.destroyAll()
            message.error('注入失败，请手动注入后再试')
            return
          }
          message.destroyAll()
        }
        
        // 根据类型执行具体操作
        if (pendingForceKillAction.value.type === 'machine') {
          message.loading('正在更换机器码...', { duration: 0 })
          await handleMachineCodeChange(true)
          operationSuccess = true
          operationMessage = i18n.value.dashboard.machineChangeSuccess
        } else if (pendingForceKillAction.value.type === 'account') {
          message.loading('正在切换账户...', { duration: 0 })
          await executeAccountSwitch(true)
          operationSuccess = true
          operationMessage = i18n.value.dashboard.accountChangeSuccess
        } else {
          message.loading('正在一键切换...', { duration: 0 })
          await executeQuickChange(true)
          operationSuccess = true
          operationMessage = i18n.value.dashboard.changeSuccess
        }
        break
        
      case 'hook':
        const hookSuccess = await autoApplyHook()
        operationSuccess = hookSuccess
        if (hookSuccess) {
          operationMessage = i18n.value.systemControl.messages.applyHookSuccess
        }
        break
    }
    
    message.destroyAll() // 清除操作中的loading消息
    
    if (operationSuccess) {
      message.success(operationMessage)
      
      // 等待一小段时间确保所有后端操作完成
      await new Promise(resolve => setTimeout(resolve, 500))
      
      // 直接启动Cursor，不再询问
      message.loading('正在启动 Cursor...', { duration: 0 })
      try {
        await launchCursor()
        message.destroyAll()
        message.success('Cursor 已启动')
      } catch (launchError) {
        message.destroyAll()
        message.error('启动 Cursor 失败: ' + (launchError instanceof Error ? launchError.message : String(launchError)))
      }
    }
  } catch (error) {
    message.destroyAll()
    message.error('操作失败: ' + (error instanceof Error ? error.message : String(error)))
  } finally {
    loading.value = false
    pendingForceKillAction.value = null
  }
}

const copyText = (text: string) => {
  if (!text) return
  navigator.clipboard.writeText(text).then(() => {
    message.success(i18n.value.common.copySuccess)
  }).catch(() => {
    message.error(i18n.value.common.copyFailed)
  })
}

// 添加版本检查相关的状态
const showUpdateModal = ref(false)
const versionInfo = ref<VersionInfo | null>(null)

// 版本比较函数
const compareVersions = (v1: string, v2: string) => {
  const parts1 = v1.split('.').map(Number)
  const parts2 = v2.split('.').map(Number)
  
  for (let i = 0; i < 3; i++) {
    if (parts1[i] > parts2[i]) return 1
    if (parts1[i] < parts2[i]) return -1
  }
  return 0
}

// 检查版本更新
const checkUpdate = async () => {
  try {
    const accessToken = localStorage.getItem('accessToken')
    if (!accessToken) return
    
    // 检查上次更新提示的时间
    const lastCheckTime = localStorage.getItem('last_version_check_time')
    const now = Date.now()
    
    if (lastCheckTime) {
      const timeDiff = now - parseInt(lastCheckTime)
      if (timeDiff < VERSION_CHECK_INTERVAL) {
        return // 如果间隔小于3小时, 不进行检查
      }
    }
    
    const remoteVersionInfo = await getVersion()
    versionInfo.value = remoteVersionInfo
    
    if (compareVersions(LOCAL_VERSION, remoteVersionInfo.version) < 0) {
      showUpdateModal.value = true
      // 只有在非强制更新时才更新检查时间
      if (!remoteVersionInfo.forceUpdate) {
        localStorage.setItem('last_version_check_time', now.toString())
      }
    }
  } catch (error) {
    console.error('检查更新失败:', error)
  }
}

// 处理下载更新
const handleDownload = async () => {
  if (versionInfo.value?.downloadUrl) {
    const url = 'https://downloader-cursor.deno.dev/'
    await open(url)
  }
}

// 处理稍后更新
const handleLater = () => {
  showUpdateModal.value = false
  // 记录关闭时间
  localStorage.setItem('last_version_check_time', Date.now().toString())
}

// 添加新的 ref
const showAdminPrivilegeModal = ref(false)

// 检查管理员权限
const checkPrivileges = async () => {
  try {
    const isAdmin = await checkAdminPrivileges();
    if (!isAdmin) {
      // 如果不是管理员，再检查是否是 Windows 平台
      const isWindows = await checkIsWindows();
      if (isWindows) {
        showAdminPrivilegeModal.value = true;
      }
    }
  } catch (error) {
    console.error('检查管理员权限失败:', error);
    message.error('检查管理员权限失败');
  }
}

// 退出程序
const handleExit = async () => {
  const appWindow = new Window('main')
  await appWindow.close()
}

// 添加更新状态 ref
const updateDisabled = ref(false)

// 在组件挂载时获取所有信息
onMounted(async () => {
  try {
    // 检查是否需要强制刷新数据
    const needRefresh = localStorage.getItem('need_refresh_dashboard')
    if (!needRefresh && (deviceInfo.value.userInfo || deviceInfo.value.cursorInfo.userInfo)) {
      return
    }
    // 清除刷新标记
    localStorage.removeItem('need_refresh_dashboard')

    loading.value = true
    // 按顺序执行
    await fetchUserInfo()
    await fetchMachineIds()
    await fetchCursorInfo()
    
    await checkPrivileges()
    await checkUpdate()
    
    // 检查更新状态
    updateDisabled.value = await checkUpdateDisabled()
    
    // 检查是否需要显示引导
    if (!localStorage.getItem('dashboard_tour_shown')) {
      // 检查accessToken是否存在
      const accessToken = localStorage.getItem('accessToken')
      
      // 只有当accessToken存在时才显示引导
      if (accessToken) {
        setTimeout(() => {
          startTour()
        }, 500)
      }
    }
  } catch (error) {
    console.error('获取信息失败:', error)
    message.error('获取信息失败')
  } finally {
    loading.value = false
  }

  // 添加事件监听器
  window.addEventListener('refresh_dashboard_data', async () => {
    try {
      loading.value = true
      await fetchUserInfo()
      await fetchMachineIds()
      await fetchCursorInfo()
    } catch (error) {
      console.error('刷新数据失败:', error)
      message.error('刷新数据失败')
    } finally {
      loading.value = false
    }
  })
})

// 修改机器码处理函数
const handleMachineCodeClick = async () => {
  try {
    machineCodeLoading.value = true
    
    // 检查 Cursor 是否在运行
    const isRunning = await checkCursorRunning()
    if (isRunning) {
      showCursorRunningModal.value = true
      pendingForceKillAction.value = { type: 'machine' }
      return
    }
    
    // 检查 CC 状态，如果未注入，直接调用注入
    if (!deviceInfo.value.hookStatus) {
      const hookSuccess = await autoApplyHook()
      
      if (hookSuccess) {
        await handleMachineCodeChange(false)
      }
      return
    }

    await handleMachineCodeChange(false)
  } catch (error) {
    console.error('机器码更换失败:', error)
    message.error('操作失败: ' + (error instanceof Error ? error.message : String(error)))
  } finally {
    machineCodeLoading.value = false
  }
}

// 添加系统检测和链接处理
const handleHistoryDownload = async () => {
  try {
    const url = 'https://downloader-cursor.deno.dev/'
    await open(url)
  } catch (error) {
    console.error('打开链接失败:', error)
    message.error('打开链接失败')
  }
}

// 添加引导相关状态
const shouldShowTour = ref(false)

// 添加积分不足模态框状态
const showInsufficientCreditsModal = ref(false)
const activationCode = ref('')
const activationLoading = ref(false)
const activationError = ref('')
const pendingCreditAction = ref<'account' | 'quick' | null>(null)

// 计算用户可用积分
const userCredits = computed(() => {
  if (!deviceInfo.value?.userInfo) {
    return 0
  }
  // 计算可用额度为总额度减去已使用额度
  return deviceInfo.value.userInfo!.totalCount - deviceInfo.value.userInfo!.usedCount
})

// 添加加载状态
const machineCodeLoading = ref(false)
const accountSwitchLoading = ref(false)
const quickChangeLoading = ref(false)

// 开始引导
const startTour = () => {
  shouldShowTour.value = true
}

// 处理引导完成
const handleTourComplete = () => {
  shouldShowTour.value = false
  localStorage.setItem('dashboard_tour_shown', 'true')
}

// 处理激活码提交
const handleActivate = async () => {
  if (!activationCode.value) {
    activationError.value = i18n.value.message.pleaseInputActivationCode
    return
  }

  try {
    activationLoading.value = true
    activationError.value = ''
    
    // 获取token
    const accessToken = localStorage.getItem('accessToken')
    if (!accessToken) {
      throw new Error('未找到Token，请重新登录')
    }
    
    // 调用激活API
    await activate(accessToken, activationCode.value)
    
    // 记录操作历史
    addHistoryRecord(
      '卡密激活',
      `激活卡密: ${activationCode.value.substring(0, 4)}****`
    )
    
    // 清空输入
    activationCode.value = ''
    
    // 关闭模态窗口
    showInsufficientCreditsModal.value = false
    
    // 刷新用户信息
    await fetchUserInfo()
    
    // 继续之前的操作
    if (pendingCreditAction.value === 'account') {
      handleAccountSwitch()
    } else if (pendingCreditAction.value === 'quick') {
      handleQuickChange()
    }
    
    pendingCreditAction.value = null
  } catch (error) {
    activationError.value = error instanceof Error ? error.message : '激活失败，请检查卡密是否正确'
  } finally {
    activationLoading.value = false
  }
}

// 监听模态框状态变化，如果有模态框显示，则隐藏引导
watch([showUpdateModal, showAdminPrivilegeModal, showCursorRunningModal], 
  ([updateModal, adminModal, cursorModal]) => {
    if (updateModal || adminModal || cursorModal) {
      shouldShowTour.value = false
    }
  }
)

// 添加表单数据
const formValue = ref({
  activationCode: ''
})

// 欢迎语
const greeting = computed(() => {
  const timeGreeting = getTimeOfDay()
  const username = deviceInfo.value.userInfo?.username || '用户'
  return `${timeGreeting}，${username}`
})
</script>

<template>
  <div class="dashboard">
    <n-space vertical :size="12">
      <n-card class="welcome-card">
        <div class="welcome-content">
          <h2 class="greeting">{{ greeting }}</h2>
          <p class="welcome-text">欢迎使用Cursor Agent代理，让我们开始愉快的编码之旅吧！</p>
        </div>
      </n-card>

      <n-space vertical size="large">
        <n-grid :cols="2" :x-gap="24" style="display: grid; grid-template-columns: repeat(2, 1fr);">
          <!-- 用户信息卡片 -->
          <n-grid-item style="display: grid;">
            <n-card :title="i18n.dashboard.userInfo" class="user-info-card" style="height: 100%; user-select: none;">
              <n-space vertical>
                <n-space vertical :size="12" style="user-select: none;">
                  <n-space :size="8" style="line-height: 1.2;" class="user-info-username">
                    <span style="width: 70px">{{ i18n.dashboard.username }}</span>
                    <n-space :size="4" align="center">
                      <span 
                        style="font-size: 14px; cursor: pointer;" 
                        @click="deviceInfo.userInfo?.username && copyText(deviceInfo.userInfo.username)"
                      >{{ deviceInfo.userInfo?.username }}</span>
                      <n-tag :type="levelMap[deviceInfo.userInfo?.level || 1].type" size="tiny" style="transform: scale(0.9)">
                        {{ levelMap[deviceInfo.userInfo?.level || 1].name }}
                      </n-tag>
                    </n-space>
                  </n-space>

                  <n-divider style="margin: 0" />

                  <n-space :size="8" style="line-height: 1.2;" class="user-info-email">
                    <span style="width: 70px">{{ i18n.dashboard.email }}</span>
                    <n-space :size="4" align="center">
                      <span 
                        style="font-size: 14px; cursor: pointer;" 
                        @click="deviceInfo.cursorInfo.userInfo?.email && copyText(deviceInfo.cursorInfo.userInfo?.email)"
                      >{{ deviceInfo.cursorInfo.userInfo?.email || '未绑定' }}</span>
                      <n-tag :type="deviceInfo.cursorInfo.userInfo?.email_verified ? 'success' : 'warning'" size="tiny" style="transform: scale(0.9)">
                        {{ deviceInfo.cursorInfo.userInfo?.email_verified ? i18n.systemControl.clientVerified : i18n.systemControl.clientUnverified }}
                      </n-tag>
                    </n-space>
                  </n-space>
                  <n-space :size="8" style="line-height: 1.2;" class="user-info-cc-status">
                    <span style="width: 70px">{{ i18n.dashboard.ccStatus }}</span>
                    <n-tag :type="deviceInfo.hookStatus === true ? 'success' : 'error'" size="small">
                      {{ deviceInfo.hookStatus === true ? i18n.systemControl.hookApplied : i18n.systemControl.hookNotApplied }}
                    </n-tag>
                  </n-space>
                  <n-space :size="8" style="line-height: 1.2;" class="user-info-register-time">
                    <span style="width: 70px">{{ i18n.dashboard.registerTime }}</span>
                    <span 
                      style="font-size: 14px; cursor: pointer;" 
                      @click="copyText(deviceInfo.cursorInfo.usage?.startOfMonth ? formatDate(deviceInfo.cursorInfo.usage.startOfMonth) : '')"
                    >{{ deviceInfo.cursorInfo.usage?.startOfMonth ? formatDate(deviceInfo.cursorInfo.usage.startOfMonth) : '未知' }}</span>
                  </n-space>
                  <span 
                    style="font-size: 12px; color: #999; word-break: break-all; text-align: center; cursor: pointer;" 
                    @click="copyText(deviceInfo.machineCode)"
                    class="user-info-machine-code"
                  >{{ deviceInfo.machineCode }}</span>
                </n-space>
              </n-space>
            </n-card>
          </n-grid-item>

          <!-- 使用统计卡片 -->
          <n-grid-item style="display: grid;">
            <n-card :title="i18n.dashboard.usageStats" style="height: 100%; user-select: none;">
              <n-space vertical :size="24" style="height: 100%; justify-content: space-around;">
                <!-- 账户使用统计 -->
                <n-space vertical :size="8" class="cursor-pool-usage">
                  <n-space justify="space-between">
                    <span>{{ i18n.dashboard.cpUsage }}</span>
                    <n-space :size="0">
                      <n-number-animation 
                        :from="0" 
                        :to="deviceInfo.userInfo?.usedCount || 0"
                        :duration="1000"
                      />
                      <span>/{{ deviceInfo.userInfo?.totalCount || 0 }}</span>
                    </n-space>
                  </n-space>
                  <n-progress
                    type="line"
                    status="success"
                    :percentage="deviceInfo.userInfo ? Math.min(100, Math.round((deviceInfo.userInfo.usedCount / (deviceInfo.userInfo.totalCount || 1)) * 100)) : 0"
                    :show-indicator="false"
                    :height="12"
                    :border-radius="6"
                    :processing="loading"
                  />
                </n-space>

                <!-- Cursor GPT-4 使用统计 -->
                <n-space vertical :size="8" class="advanced-model-usage">
                  <n-space justify="space-between">
                    <span>{{ i18n.dashboard.advancedModelUsage }}</span>
                    <n-space v-if="deviceInfo.cursorInfo.usage" :size="0">
                      <n-number-animation 
                        :from="0"
                        :to="deviceInfo.cursorInfo.usage['gpt-4']?.numRequests || 0"
                        :duration="1000"
                      />
                      <span>/{{ deviceInfo.cursorInfo.usage['gpt-4']?.maxRequestUsage || 0 }}</span>
                    </n-space>
                    <span v-else>{{ i18n.dashboard.cannotGetUsage }}</span>
                  </n-space>
                  <n-progress
                    type="line"
                    status="success"
                    :percentage="cursorGpt4Percentage"
                    :show-indicator="false"
                    :height="12"
                    :border-radius="6"
                    :processing="loading"
                  />
                </n-space>

                <!-- Cursor GPT-3.5 使用统计 -->
                <n-space vertical :size="8" class="basic-model-usage">
                  <n-space justify="space-between">
                    <span>{{ i18n.dashboard.basicModelUsage }}</span>
                    <n-space v-if="deviceInfo.cursorInfo.usage" :size="0">
                      <n-number-animation 
                        :from="0" 
                        :to="deviceInfo.cursorInfo.usage['gpt-3.5-turbo']?.numRequests || 0"
                        :duration="1000"
                      />
                      <span v-if="deviceInfo.cursorInfo.usage['gpt-3.5-turbo']?.maxRequestUsage">
                        /{{ deviceInfo.cursorInfo.usage['gpt-3.5-turbo']?.maxRequestUsage }}
                      </span>
                      <span v-else>/{{ i18n.dashboard.unlimited }}</span>
                    </n-space>
                    <span v-else>{{ i18n.dashboard.cannotGetUsage }}</span>
                  </n-space>
                  <n-progress
                    type="line"
                    status="success"
                    :percentage="cursorGpt35Percentage"
                    :show-indicator="false"
                    :height="12"
                    :border-radius="6"
                    :processing="loading"
                  />
                </n-space>

              </n-space>
            </n-card>
          </n-grid-item>
        </n-grid>

        <!-- 快捷操作卡片 -->
        <n-card :title="i18n.dashboard.quickActions" class="quick-actions-card" style="user-select: none;">
          <n-space vertical>
            <n-space justify="space-around">
              <n-button type="primary" @click="handleQuickChange" :disabled="!deviceInfo.userInfo" :loading="quickChangeLoading">
                {{ i18n.dashboard.quickChange }}
              </n-button>
              <n-button type="primary" @click="handleAccountSwitch" :disabled="!deviceInfo.userInfo" :loading="accountSwitchLoading">
                {{ i18n.dashboard.changeAccount }}
              </n-button>
              <n-button type="primary" @click="handleMachineCodeClick" :loading="machineCodeLoading">
                {{ i18n.dashboard.changeMachineCode }}
              </n-button>
              <n-button type="primary" @click="handleHistoryDownload">
                {{ i18n.dashboard.cursorHistoryDownload }}
              </n-button>
            </n-space>
          </n-space>
        </n-card>

        <!-- 添加更新模态框 -->
        <n-modal
          v-model:show="showUpdateModal"
          :mask-closable="!versionInfo?.forceUpdate"
          :closable="!versionInfo?.forceUpdate"
          preset="card"
          style="width: 500px"
          :title="i18n.dashboard.newVersionAvailable"
        >
          <n-space vertical>
            <div>{{ i18n.dashboard.currentVersion }}: {{ LOCAL_VERSION }}</div>
            <div>{{ i18n.dashboard.newVersion }}: {{ versionInfo?.version }}</div>
            <n-divider />
            <div style="white-space: pre-line">{{ versionInfo?.changeLog }}</div>
            <n-space justify="end">
              <n-button
                v-if="!versionInfo?.forceUpdate"
                @click="handleLater"
              >
                {{ i18n.dashboard.later }}
              </n-button>
              <n-button
                type="primary"
                @click="handleDownload"
              >
                {{ i18n.dashboard.downloadNow }}
              </n-button>
            </n-space>
          </n-space>
        </n-modal>

        <n-modal
          v-model:show="showUnusedCreditsModal"
          preset="dialog"
          title="使用提醒"
          :closable="true"
          :mask-closable="false"
        >
          <template #default>
            <p>您还有 {{ unusedCredits }} 次高级模型使用次数未使用</p>
            <p style="margin-top: 12px; color: #666;">
              {{ pendingAction === 'quick' ? '切换代理账号（账户+机器码）将扣除50积分' : '切换代理账号将扣除50积分' }}，确定要继续吗？
            </p>
          </template>
          <template #action>
            <n-space justify="end">
              <n-button @click="handleCancelSwitch">
                取消
              </n-button>
              <n-button type="primary" @click="handleConfirmSwitch">
                确认切换
              </n-button>
            </n-space>
          </template>
        </n-modal>

        <!-- 添加 Cursor 运行提醒模态框 -->
        <n-modal
          v-model:show="showCursorRunningModal"
          preset="dialog"
          title="Cursor 正在运行"
          :closable="true"
          :mask-closable="false"
        >
          <template #default>
            检测到 Cursor 正在运行, 请保存尚未更改的项目再继续操作!
          </template>
          <template #action>
            <n-space justify="end">
              <n-button type="warning" @click="handleForceKill">
                我已保存, 强制关闭
              </n-button>
            </n-space>
          </template>
        </n-modal>

        <!-- 添加管理员权限提示模态框 -->
        <n-modal
          v-model:show="showAdminPrivilegeModal"
          preset="dialog"
          title="需要管理员权限"
          :closable="false"
          :mask-closable="false"
          style="width: 500px"
        >
          <template #header>
            <n-space align="center">
              <n-icon size="24" color="#f0a020">
                <warning-outlined />
              </n-icon>
              <span>需要管理员权限</span>
            </n-space>
          </template>
          <div style="margin: 24px 0;">
            <p>本程序需要管理员权限才能正常运行。</p>
            <p style="margin-top: 12px; color: #999;">
              请右键点击程序图标,选择"以管理员身份运行"后重新启动程序。
            </p>
          </div>
          <template #action>
            <n-button type="error" @click="handleExit" block>
              退出程序
            </n-button>
          </template>
        </n-modal>

        <!-- 修改积分不足模态框 -->
        <n-modal
          v-model:show="showInsufficientCreditsModal"
          preset="dialog"
          title="额度不足"
          :closable="true"
          :mask-closable="false"
          style="width: 500px"
        >
          <n-form
            :model="formValue"
            label-placement="left"
            label-width="auto"
            require-mark-placement="right-hanging"
          >
            <div style="margin-bottom: 16px">
              <p>您当前对话额度不足，账户切换需要消耗50额度。</p>
              <p style="margin-top: 12px; color: #ff4d4f;">
                当前可用额度: {{ deviceInfo.userInfo ? deviceInfo.userInfo.totalCount - deviceInfo.userInfo.usedCount : 0 }}，还需要: {{ Math.max(0, 50 - (deviceInfo.userInfo ? deviceInfo.userInfo.totalCount - deviceInfo.userInfo.usedCount : 0)) }} 额度
              </p>
            </div>
            
            <n-form-item label="激活码">
              <n-input
                v-model:value="activationCode"
                type="text"
                placeholder="请输入卡密"
                :disabled="activationLoading"
              />
            </n-form-item>
            
            <p v-if="activationError" style="color: #ff4d4f; margin-top: 8px;">
              {{ activationError }}
            </p>
          </n-form>

          <template #action>
            <n-space justify="end">
              <n-button @click="showInsufficientCreditsModal = false" :disabled="activationLoading">
                取消
              </n-button>
              <n-button type="primary" @click="handleActivate" :loading="activationLoading">
                激活卡密
              </n-button>
            </n-space>
          </template>
        </n-modal>

        <!-- 添加引导组件 -->
        <DashboardTour :show="shouldShowTour" :onComplete="handleTourComplete" />
      </n-space>
    </n-space>
  </div>
</template>

<style scoped>
/* 添加样式确保 grid 项目高度一致 */
.n-grid {
  grid-auto-rows: 1fr;
}

.n-grid-item {
  min-height: 0;
}

.welcome-card {
  background: linear-gradient(135deg, #4CAF50 0%, #2196F3 100%);
  color: white;
}

.welcome-content {
  padding: 16px;
}

.greeting {
  font-size: 24px;
  margin: 0 0 8px 0;
}

.welcome-text {
  font-size: 16px;
  margin: 0;
  opacity: 0.9;
}
</style>