<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { 
  NCard, 
  NForm, 
  NFormItem, 
  NInput, 
  NButton, 
  useMessage,
  NSpace
  // 注释掉未使用的组件
  // NModal
} from 'naive-ui'
import { login, register, getTenantId } from '../api'
// 注释掉未使用的函数
// import { resetPassword } from '../api'
import type { LoginRequest, RegisterRequest } from '../api/types'
import { useI18n } from '../locales'
import { messages } from '../locales/messages'
import type { HTMLAttributes } from 'vue'
import { encrypt } from '../utils/crypto'
// 注释掉未使用的函数
// import { sha1 } from '../utils/crypto'

const message = useMessage()
const loading = ref(false)

const formData = ref({
  username: '',
  password: '',
  tenant_id: '',
})

// 检查用户是否存在的防抖定时器
let checkUserTimer: number | null = null

// 添加注册模式状态
const isRegisterMode = ref(false)

// 计算标题
const formTitle = computed(() => messages[currentLang.value].login[isRegisterMode.value ? 'registerButton' : 'title'])

// 计算按钮文本
const buttonText = computed(() => messages[currentLang.value].login[isRegisterMode.value ? 'registerButton' : 'loginButton'])

// 添加一个变量保存当前用户的tenantId
const currentTenantId = ref<string | null>(null);

// 切换模式
function toggleMode() {
  isRegisterMode.value = !isRegisterMode.value
  // 清空表单
  formData.value = {
    username: '',
    password: '',
    tenant_id: '',
  }
  // 清除tenantId
  currentTenantId.value = null;
}

// 修改监听用户名变化的逻辑
watch(() => formData.value.username, async (newValue) => {
  if (!newValue) {
    currentTenantId.value = null;
    return
  }
  
  // 防抖处理
  if (checkUserTimer) clearTimeout(checkUserTimer)
  checkUserTimer = setTimeout(async () => {
    try {
      // 使用getTenantId替代checkUser
      const tenantId = await getTenantId(newValue);
      console.log('获取到tenantId:', tenantId);
      
      // 保存tenantId
      currentTenantId.value = tenantId;
      
      // 如果是注册模式，用户存在时自动切换到登录
      if (isRegisterMode.value) {
        message.info(messages[currentLang.value].login.userExists);
        isRegisterMode.value = false;
      }
    } catch (error) {
      console.error('查找用户失败:', error);
      
      // 清除tenantId
      currentTenantId.value = null;
      
      // 如果是登录模式，用户不存在时提示
      if (!isRegisterMode.value) {
        message.error(messages[currentLang.value].login.userNotExists);
      }
    }
  }, 500)
})

// 添加密码验证正则
const passwordRegex = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d]{8,}$/

// 添加密码输入状态
const passwordInputStatus = computed(() => {
  const password = formData.value.password
  if (!password) return undefined
  if (!passwordRegex.test(password)) return 'error'
  return undefined
})

// 添加密码输入状态和提示信息
const passwordInputFeedback = computed(() => {
  const password = formData.value.password
  if (!password) return ''
  if (!passwordRegex.test(password)) {
    return messages[currentLang.value].login.passwordInvalid
  }
  return ''
})

// 修改处理提交的逻辑, 添加密码验证
async function handleSubmit() {
  if (!formData.value.username) {
    message.error(messages[currentLang.value].login.emailError)
    return
  }

  if (!formData.value.password) {
    message.error(messages[currentLang.value].login.passwordInvalid)
    return
  }

  // 注册模式下校验码必填
  if (isRegisterMode.value && !formData.value.tenant_id) {
    message.error('请输入校验码')
    return
  }

  try {
    loading.value = true
    const deviceId = 'device-' + Math.random().toString(36).substr(2, 9)
    
    if (isRegisterMode.value) {
      // 注册逻辑
      const registerParams: RegisterRequest = {
        tenantId: formData.value.tenant_id,
        account: formData.value.username,
        password: formData.value.password // 不加密密码
      }

      console.log('注册参数:', registerParams); // 添加日志查看请求参数

      const result = await register(registerParams)
      if (result) {
        // 注册成功，直接显示成功消息
        message.success('注册成功')
        // 切换到登录模式
        isRegisterMode.value = false
        // 清空表单
        formData.value = {
          username: registerParams.account, // 保留用户名，方便用户登录
          password: '',
          tenant_id: '',
        }
      } else {
        message.error('注册失败')
      }
    } else {
      // 登录逻辑
      try {
        // 检查是否已有tenantId，如果没有则获取
        let tenantId = currentTenantId.value;
        if (!tenantId) {
          try {
            tenantId = await getTenantId(formData.value.username);
            console.log('获取到tenantId:', tenantId);
          } catch (error) {
            console.error('查找用户失败:', error);
            message.error(messages[currentLang.value].login.userNotExists);
            return;
          }
        }
        
        // 然后登录
        const loginParams: LoginRequest = {
          username: formData.value.username,
          password: encrypt(formData.value.password), // 使用SM2加密
          deviceId: deviceId,
          tenantId: tenantId // 使用tenantId
        }

        const result = await login(loginParams)
        if (result.accessToken) {
          // 不再需要设置apiKey，因为login函数已经保存了token
          message.success(messages[currentLang.value].login.loginSuccess)
          emit('login-success')
          // 手动触发刷新数据
          window.dispatchEvent(new CustomEvent('refresh_dashboard_data'))
        } else {
          message.error(messages[currentLang.value].login.loginFailed)
        }
      } catch (error) {
        console.error('登录失败:', error);
        message.error(error instanceof Error ? error.message : messages[currentLang.value].login.loginFailed);
      }
    }
  } catch (error) {
    message.error(isRegisterMode.value ? '注册失败' : messages[currentLang.value].login.loginFailed + ': ' + (error instanceof Error ? error.message : ''))
  } finally {
    loading.value = false
  }
}

const emit = defineEmits(['login-success'])

const { currentLang, i18n } = useI18n()

// 定义自定义输入属性类型
interface CustomInputProps extends HTMLAttributes {
  autocomplete?: string
  'data-form-type'?: string
  'data-lpignore'?: string
}

// 定义输入属性
const inputProps = {
  autocomplete: 'off',
  'data-form-type': 'other',
  'data-lpignore': 'true'
} as CustomInputProps

// 注释掉忘记密码相关状态和函数
/*
const showForgotPassword = ref(false)
const forgotPasswordLoading = ref(false)
const forgotPasswordForm = ref({
  email: '',
  newPassword: '',
  confirmPassword: ''
})

// 处理忘记密码提交
const handleForgotPassword = async () => {
  if (!forgotPasswordForm.value.email) {
    message.error('请输入邮箱地址')
    return
  }

  if (!forgotPasswordForm.value.newPassword || !passwordRegex.test(forgotPasswordForm.value.newPassword)) {
    message.error('新密码不符合要求')
    return
  }

  if (forgotPasswordForm.value.newPassword !== forgotPasswordForm.value.confirmPassword) {
    message.error('两次输入的密码不一致')
    return
  }

  try {
    forgotPasswordLoading.value = true
    const result = await resetPassword(
      forgotPasswordForm.value.email,
      '', // 空验证码
      sha1(forgotPasswordForm.value.newPassword) // 保持密码加密
    )
    
    // 处理成功响应
    message.success(result || '密码重置成功')
    showForgotPassword.value = false
  } catch (error) {
    message.error(error instanceof Error ? error.message : '密码重置失败')
  } finally {
    forgotPasswordLoading.value = false
  }
}
*/
</script>

<template>
  <!-- 添加一个隐藏的假表单来欺骗浏览器的自动填充 -->
  <form style="display: none" aria-hidden="true">
    <input type="text" />
    <input type="email" />
    <input type="password" />
  </form>

  <div class="login-overlay">
    <n-card :title="formTitle" class="login-card">
      <n-form>
        <n-form-item :label="i18n.login.emailPlaceholder">
          <n-input
            v-model:value="formData.username"
            :placeholder="i18n.login.emailPlaceholder"
            :disabled="loading"
            autocomplete="off"
            :input-props="inputProps"
          />
        </n-form-item>
        
        <n-form-item 
          :label="i18n.login.passwordPlaceholder"
          :status="passwordInputStatus"
        >
          <n-input 
            v-model:value="formData.password"
            type="password"
            :placeholder="i18n.login.passwordPlaceholder"
            :disabled="loading"
          />
          <template #feedback>
            {{ passwordInputFeedback }}
          </template>
        </n-form-item>

        <!-- 注册模式下显示校验码输入框 -->
        <n-form-item v-if="isRegisterMode" label="校验码">
          <n-input 
            v-model:value="formData.tenant_id"
            :placeholder="'请输入校验码'"
            :disabled="loading"
          />
        </n-form-item>

        <n-space vertical :size="12">
          <n-button 
            type="primary" 
            block 
            @click="handleSubmit"
            :loading="loading"
          >
            {{ buttonText }}
          </n-button>

          <n-space justify="space-between">
            <n-button
              text
              tag="a"
              @click="toggleMode"
              :disabled="loading"
            >
              {{ isRegisterMode 
                ? i18n.login.hasAccount 
                : `${i18n.login.noAccount} ${i18n.login.register}`
              }}
            </n-button>
            <!-- 注释掉忘记密码按钮
            <n-button
              v-if="!isRegisterMode"
              text
              tag="a"
              @click="showForgotPassword = true"
              :disabled="loading"
            >
              {{ i18n.common.forgotPassword }}
            </n-button>
            -->
          </n-space>
        </n-space>
      </n-form>
    </n-card>
  </div>

  <!-- 注释掉忘记密码模态框
  <n-modal v-model:show="showForgotPassword">
    <n-card style="width: 400px" title="忘记密码">
      <n-form>
        <n-form-item label="邮箱">
          <n-input
            v-model:value="forgotPasswordForm.email"
            placeholder="请输入注册邮箱"
            :disabled="forgotPasswordLoading"
          />
        </n-form-item>

        <n-form-item label="新密码">
          <n-input
            v-model:value="forgotPasswordForm.newPassword"
            type="password"
            placeholder="请输入新密码"
            :disabled="forgotPasswordLoading"
          />
        </n-form-item>

        <n-form-item label="确认密码">
          <n-input
            v-model:value="forgotPasswordForm.confirmPassword"
            type="password"
            placeholder="请再次输入新密码"
            :disabled="forgotPasswordLoading"
          />
        </n-form-item>

        <n-space justify="end">
          <n-button @click="showForgotPassword = false">取消</n-button>
          <n-button
            type="primary"
            @click="handleForgotPassword"
            :loading="forgotPasswordLoading"
          >
            重置密码
          </n-button>
        </n-space>
      </n-form>
    </n-card>
  </n-modal>
  -->
</template>

<style scoped>
.login-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.2);
  backdrop-filter: blur(10px);
  z-index: 1000;
  user-select: none;
}

.login-card {
  width: 400px;
  max-width: 90%;
}

:deep(.n-card) {
  background: var(--n-color);
  color: var(--n-text-color);
}

:deep(.n-card-header) {
  text-align: center;
  font-size: 1.5em;
}

:deep(.n-input) {
  user-select: text;
}

:deep(.n-input-wrapper) {
  user-select: text;
}

:deep(.n-form-item-feedback-wrapper) {
  min-height: 20px;
}

:deep(.n-form-item-feedback) {
  color: var(--n-feedback-text-color);
  font-size: 12px;
}

/* 添加以下样式来进一步防止自动填充 */
:deep(.n-input__input-el) {
  /* 禁用 webkit 浏览器的自动填充样式 */
  &:-webkit-autofill,
  &:-webkit-autofill:hover,
  &:-webkit-autofill:focus,
  &:-webkit-autofill:active {
    -webkit-box-shadow: 0 0 0 30px var(--n-color) inset !important;
    -webkit-text-fill-color: var(--n-text-color) !important;
    transition: background-color 5000s ease-in-out 0s;
  }
}
</style>
