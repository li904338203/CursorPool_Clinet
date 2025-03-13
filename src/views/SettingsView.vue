<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { 
  NCard, 
  NSpace, 
  NForm, 
  NFormItem, 
  NInput, 
  NButton,
  NInputGroup,
  NModal,
  useMessage
} from 'naive-ui'
import { useI18n } from '../locales'
import { messages } from '../locales/messages'
import LanguageSwitch from '../components/LanguageSwitch.vue'
import { 
  changePassword, 
  activate, 
  checkCursorRunning,
  disableCursorUpdate,
  restoreCursorUpdate,
  checkUpdateDisabled,
  checkHookStatus,
  applyHook,
  restoreHook
} from '@/api'
import { addHistoryRecord } from '../utils/history'
import { version } from '../../package.json'

const message = useMessage()
const { currentLang, i18n } = useI18n()

interface SettingsForm {
  activationCode: string
  currentPassword: string
  newPassword: string
  confirmPassword: string
}

const formValue = ref<SettingsForm>({
  activationCode: '',
  currentPassword: '',
  newPassword: '',
  confirmPassword: ''
})

// 修改控制状态
const controlStatus = ref({
  updateDisabled: false,
  isHooked: false
})

// 为每个操作添加单独的加载状态
const disableUpdateLoading = ref(false)
const restoreUpdateLoading = ref(false)
const applyHookLoading = ref(false)
const restoreHookLoading = ref(false)

const showControlRunningModal = ref(false)
const pendingControlAction = ref<'disableUpdate' | 'restoreUpdate' | 'applyHook' | 'restoreHook' | null>(null)

// 为激活和修改密码添加独立的加载状态
const activateLoading = ref(false)
const passwordChangeLoading = ref(false)

// 添加额度迁移相关状态
const creditTransferLoading = ref(false)

// 添加额度迁移确认框状态
const showCreditTransferModal = ref(false)
const creditTransferConfirmLoading = ref(false)

// 处理退出登录
const handleLogout = async () => {
  try {
    // 清除所有用户相关数据
    localStorage.removeItem('accessToken')
    localStorage.removeItem('userInfo')
    localStorage.removeItem('last_version_check_time')
    localStorage.removeItem('need_refresh_dashboard')
    
    // 触发刷新事件
    window.dispatchEvent(new CustomEvent('refresh_dashboard_data'))
    
    message.success(i18n.value.common.logout + '成功')
    
    // 强制刷新页面
    window.location.reload()
  } catch (error) {
    console.error('退出登录失败:', error)
    message.error('退出登录失败，请刷新页面重试')
  }
}

const handlePasswordChange = async () => {
  if (!formValue.value.currentPassword || !formValue.value.newPassword || !formValue.value.confirmPassword) {
    message.warning(messages[currentLang.value].message.pleaseInputPassword)
    return
  }
  if (formValue.value.newPassword !== formValue.value.confirmPassword) {
    message.error(messages[currentLang.value].message.passwordNotMatch)
    return
  }

  passwordChangeLoading.value = true
  try {
    const accessToken = localStorage.getItem('accessToken')
    if (!accessToken) {
      throw new Error('未找到 Token')
    }

    const result = await changePassword(accessToken, formValue.value.currentPassword, formValue.value.newPassword)
    if (result) {
      message.success(messages[currentLang.value].message.passwordChangeSuccess)
      addHistoryRecord(
        '密码修改',
        '成功修改密码'
      )
      formValue.value.currentPassword = ''
      formValue.value.newPassword = ''
      formValue.value.confirmPassword = ''
      
      await handleLogout()
    }
  } catch (error) {
    message.error(error instanceof Error ? error.message : messages[currentLang.value].message.passwordChangeFailed)
  } finally {
    passwordChangeLoading.value = false
  }
}

const handleActivate = async () => {
  if (!formValue.value.activationCode) {
    message.warning(messages[currentLang.value].message.pleaseInputActivationCode)
    return
  }

  activateLoading.value = true
  try {
    const accessToken = localStorage.getItem('accessToken')
    if (!accessToken) {
      throw new Error('未找到 Token')
    }

    await activate(accessToken, formValue.value.activationCode)
    message.success(messages[currentLang.value].message.activationSuccess)
    addHistoryRecord(
      '激活码兑换',
      '成功兑换激活码'
    )
    formValue.value.activationCode = ''
  } catch (error) {
    console.error('激活失败:', error)
    message.error(messages[currentLang.value].message.activationFailed)
  } finally {
    activateLoading.value = false
  }
}

// 检查控制状态
const checkControlStatus = async () => {
  try {
    controlStatus.value.updateDisabled = await checkUpdateDisabled()
    controlStatus.value.isHooked = await checkHookStatus()
  } catch (error) {
    console.error('检查控制状态失败:', error)
  }
}

// 修改 handleControlAction 函数
const handleControlAction = async (action: 'disableUpdate' | 'restoreUpdate' | 'applyHook' | 'restoreHook', force_kill: boolean = false) => {
  // 根据操作设置对应的加载状态
  const loadingRef = {
    'disableUpdate': disableUpdateLoading,
    'restoreUpdate': restoreUpdateLoading,
    'applyHook': applyHookLoading,
    'restoreHook': restoreHookLoading
  }[action]

  try {
    loadingRef.value = true
    if (!force_kill) {
      const isRunning = await checkCursorRunning()
      if (isRunning) {
        showControlRunningModal.value = true
        pendingControlAction.value = action
        return
      }
    }

    let successMessage = ''
    let historyAction = ''
    
    switch (action) {
      case 'disableUpdate':
        await disableCursorUpdate(force_kill)
        successMessage = messages[currentLang.value].systemControl.messages.disableUpdateSuccess
        historyAction = messages[currentLang.value].systemControl.history.disableUpdate
        controlStatus.value.updateDisabled = true
        break
      case 'restoreUpdate':
        await restoreCursorUpdate(force_kill)
        successMessage = messages[currentLang.value].systemControl.messages.restoreUpdateSuccess
        historyAction = messages[currentLang.value].systemControl.history.restoreUpdate
        controlStatus.value.updateDisabled = false
        break
      case 'applyHook':
        await applyHook(force_kill)
        successMessage = messages[currentLang.value].systemControl.messages.applyHookSuccess
        historyAction = messages[currentLang.value].systemControl.history.applyHook
        controlStatus.value.isHooked = true
        break
      case 'restoreHook':
        await restoreHook(force_kill)
        successMessage = messages[currentLang.value].systemControl.messages.restoreHookSuccess
        historyAction = messages[currentLang.value].systemControl.history.restoreHook
        controlStatus.value.isHooked = false
        break
    }

    message.success(successMessage)
    showControlRunningModal.value = false
    addHistoryRecord('系统控制', historyAction)
  } catch (error) {
    message.error(error instanceof Error ? error.message : '操作失败')
  } finally {
    loadingRef.value = false
  }
}

// 处理强制关闭
const handleControlForceKill = async () => {
  if (pendingControlAction.value) {
    await handleControlAction(pendingControlAction.value, true)
  }
}

// 处理额度迁移
const handleCreditTransfer = async () => {
  showCreditTransferModal.value = true
}

// 确认额度迁移
const confirmCreditTransfer = async () => {
  try {
    creditTransferConfirmLoading.value = true
    
    // 获取token
    const token = localStorage.getItem('accessToken')
    if (!token) {
      throw new Error('未找到登录凭证，请重新登录')
    }
    
    // 调用服务器接口
    const response = await fetch('http://27.25.153.228:8080/api/blade-system/cardKey/balanceToCardKey', {
      method: 'POST',
      headers: {
        'Blade-Auth': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    })
    
    const data = await response.json()
    
    if (data.success && data.code === 200) {
      message.success(messages[currentLang.value].message.creditTransferSuccess)
      // 关闭确认框
      setTimeout(() => {
        showCreditTransferModal.value = false
      }, 1000)
    } else {
      // 显示错误信息
      message.error(data.msg || '额度迁移失败')
    }
  } catch (error) {
    console.error('额度迁移失败:', error)
    message.error('额度迁移失败: ' + (error instanceof Error ? error.message : String(error)))
  } finally {
    creditTransferConfirmLoading.value = false
  }
}

// 在组件挂载时检查控制状态
onMounted(async () => {
  await checkControlStatus()
})
</script>

<template>
  <n-space vertical :size="24">
    <n-card :title="i18n.systemControl.title">
      <n-space vertical>
        <!-- Hook 控制部分 -->
        <n-space justify="space-between" align="center">
          <span>{{ i18n.systemControl.hookStatus }}: {{ controlStatus.isHooked ? i18n.systemControl.hookApplied : i18n.systemControl.hookNotApplied }}</span>
          <n-space>
            <n-button 
              type="warning" 
              :loading="applyHookLoading"
              :disabled="controlStatus.isHooked"
              @click="handleControlAction('applyHook')"
              style="width: 120px"
            >
              {{ i18n.systemControl.applyHook }}
            </n-button>
            <n-button 
              type="primary"
              :loading="restoreHookLoading"
              :disabled="!controlStatus.isHooked"
              @click="handleControlAction('restoreHook')"
              style="width: 120px"
            >
              {{ i18n.systemControl.restoreHook }}
            </n-button>
          </n-space>
        </n-space>

        <!-- 更新控制部分 -->
        <!-- <n-space justify="space-between" align="center">
          <span>{{ i18n.systemControl.updateStatus }}: {{ controlStatus.updateDisabled ? i18n.systemControl.updateDisabled : i18n.systemControl.updateEnabled }}</span>
          <n-space>
            <n-button 
              type="warning" 
              :loading="disableUpdateLoading"
              :disabled="controlStatus.updateDisabled"
              @click="handleControlAction('disableUpdate')"
              style="width: 120px"
            >
              {{ i18n.systemControl.disableUpdate }}
            </n-button>
            <n-button 
              type="primary"
              :loading="restoreUpdateLoading"
              :disabled="!controlStatus.updateDisabled"
              @click="handleControlAction('restoreUpdate')"
              style="width: 120px"
            >
              {{ i18n.systemControl.restoreUpdate }}
            </n-button>
          </n-space>
        </n-space> -->
      </n-space>
    </n-card>

    <n-card :title="messages[currentLang].settings.activation">
      <n-form
        :model="formValue"
        label-placement="left"
        label-width="120"
        require-mark-placement="right-hanging"
      >
        <n-form-item
          :label="messages[currentLang].settings.activationCode"
          path="activationCode"
        >
          <n-space align="center" :size="15">
            <n-input-group style="width: 360px">
              <n-input
                v-model:value="formValue.activationCode"
                :placeholder="messages[currentLang].settings.activationCode"
                size="large"
              />
              <n-button
                type="primary"
                @click="handleActivate"
                :loading="activateLoading"
                size="large"
              >
                {{ messages[currentLang].settings.activate }}
              </n-button>
            </n-input-group>
            
            <n-button
              type="info"
              @click="handleCreditTransfer"
              :loading="creditTransferLoading"
              size="large"
            >
              {{ messages[currentLang].settings.creditTransfer }}
            </n-button>
          </n-space>
        </n-form-item>

        <n-form-item>
          <div style="padding-left: 40px">
            <n-button
              type="error"
              @click="handleLogout"
              size="large"
            >
              {{ i18n.common.logout }}
            </n-button>
          </div>
        </n-form-item>
      </n-form>
    </n-card>

    <n-card :title="messages[currentLang].settings.changePassword">
      <n-form
        :model="formValue"
        label-placement="left"
        label-width="100"
        require-mark-placement="right-hanging"
      >
        <n-form-item :label="messages[currentLang].settings.currentPassword">
          <n-input
            v-model:value="formValue.currentPassword"
            type="password"
            show-password-on="click"
            :placeholder="messages[currentLang].settings.currentPassword"
            maxlength="20"
            minlength="6"
            :allow-input="(value) => /^[a-zA-Z0-9]*$/.test(value)"
          />
        </n-form-item>

        <n-form-item :label="messages[currentLang].settings.newPassword">
          <n-input
            v-model:value="formValue.newPassword"
            type="password"
            show-password-on="click"
            :placeholder="messages[currentLang].settings.newPassword"
            maxlength="20"
            minlength="6"
            :allow-input="(value) => /^[a-zA-Z0-9]*$/.test(value)"
          />
        </n-form-item>

        <n-form-item :label="messages[currentLang].settings.confirmPassword">
          <n-input
            v-model:value="formValue.confirmPassword"
            type="password"
            show-password-on="click"
            :placeholder="messages[currentLang].settings.confirmPassword"
            maxlength="20"
            minlength="6"
            :allow-input="(value) => /^[a-zA-Z0-9]*$/.test(value)"
          />
        </n-form-item>

        <div style="margin-top: 24px">
          <n-button 
            type="primary" 
            @click="handlePasswordChange"
            :loading="passwordChangeLoading"
          >
            {{ messages[currentLang].settings.changePassword }}
          </n-button>
        </div>
      </n-form>
    </n-card>

    <n-card title="Language / 语言">
      <n-space vertical>
        <language-switch />
      </n-space>
    </n-card>

    <n-card :title="messages[currentLang].settings.about">
      <n-space vertical :size="12">
        <p>{{ i18n.about.appName }} v{{ version }}</p>
      </n-space>
    </n-card>

    <!-- 合并后的 Cursor 运行提醒模态框 -->
    <n-modal
      v-model:show="showControlRunningModal"
      preset="dialog"
      title="提示"
      :closable="false"
      :mask-closable="false"
    >
      <template #default>
        {{ i18n.systemControl.messages.cursorRunning }}
      </template>
      <template #action>
        <n-space justify="end">
          <n-button type="warning" @click="handleControlForceKill">
            {{ i18n.systemControl.messages.forceKillConfirm }}
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- 额度迁移确认框 -->
    <n-modal
      v-model:show="showCreditTransferModal"
      preset="dialog"
      title="确认额度迁移"
      :closable="true"
      :mask-closable="true"
    >
      <template #default>
        是否把老插件的额度迁移到本软件中，额度一旦迁移无法恢复，是否操作？
      </template>
      <template #action>
        <n-space justify="end">
          <n-button @click="showCreditTransferModal = false">
            {{ i18n.systemControl.messages.cancel }}
          </n-button>
          <n-button type="primary" @click="confirmCreditTransfer" :loading="creditTransferConfirmLoading">
            {{ i18n.systemControl.messages.confirm }}
          </n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>