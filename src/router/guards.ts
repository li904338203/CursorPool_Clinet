import type { Router, RouteLocationNormalized, NavigationGuardNext } from 'vue-router'

export function setupRouterGuards(router: Router) {
  router.beforeEach((
    to: RouteLocationNormalized,
    _from: RouteLocationNormalized,
    next: NavigationGuardNext
  ) => {
    const accessToken = localStorage.getItem('accessToken')
    
    if (to.name !== 'login' && !accessToken) {
      next({ name: 'login' })
    } else if (to.name === 'login' && accessToken) {
      next({ name: 'dashboard' })
    } else {
      next()
    }
  })
}