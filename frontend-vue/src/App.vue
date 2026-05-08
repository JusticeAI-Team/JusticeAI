<template>
  <el-container class="hud-layout-container">
    <!-- 1. 侧边栏：明亮政务/科研风格 -->
    <el-aside width="240px" class="hud-aside">
      <div class="logo-box">
        <span class="logo-icon"></span>
        <span class="logo-text">数智检察预警平台</span>
      </div>
      
      <el-menu
        active-text-color="#122E8A"
        background-color="transparent"
        text-color="#666666"
        :default-active="activeMenu"
        class="hud-menu"
        @select="handleMenuSelect"
      >
        <el-menu-item index="1">
          <i class="el-icon-data-board"></i>
          <span>全景风险指挥大屏</span>
        </el-menu-item>
        <el-menu-item index="2">
          <i class="el-icon-cpu"></i>
          <span>检察官智能工作台</span>
        </el-menu-item>
        <el-menu-item index="3">
          <i class="el-icon-warning-outline"></i>
          <span>线索审核与预警</span>
        </el-menu-item>
        <el-menu-item index="4">
          <i class="el-icon-document"></i>
          <span>文书辅助生成</span>
        </el-menu-item>
        <el-menu-item index="5">
          <i class="el-icon-connection"></i>
          <span>异构数据接入</span>
        </el-menu-item>
      </el-menu>
      
      <!-- 左下角系统版本标识 -->
      <div class="aside-footer">
        <div class="sys-version">SYS.VER: GLM-5.1.0</div>
        <div class="sys-status"><span class="blink-dot"></span> CONNECTION SECURE</div>
      </div>
    </el-aside>

    <el-container>
      <!-- 2. 顶部 Header：明亮指挥中心风格 -->
      <el-header class="hud-header">
        <div class="header-left">
          <span class="title-decorator"></span>
          <span class="main-title">基层社会治理重点领域风险研判一体化平台</span>
        </div>
        
        <div class="header-right">
       
          <el-dropdown trigger="click">
            <div class="user-profile">
              <div class="avatar-box">检</div>
              <span class="user-name">检察官：测试用户</span>
            </div>
            <template #dropdown>
              <el-dropdown-menu class="hud-dropdown">
                <el-dropdown-item>个人中心</el-dropdown-item>
                <el-dropdown-item>系统设置</el-dropdown-item>
                <el-dropdown-item divided style="color: #D9363E; font-weight: bold;">退出终端</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>
      
      <!-- 3. 主体内容区：你之前改好的柔奶白页面会在这里渲染 -->
      <el-main class="hud-main">
        <div class="main-content-wrapper">
          <AgentWorkspace v-if="activeMenu === '2'" />
          <WarningCenter v-else-if="activeMenu === '3'" />
          <DocumentAssistant v-else-if="activeMenu === '4'" />
          <DataIntegration v-else-if="activeMenu === '5'" />
          <Dashboard v-else />
        </div>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup>
import { ref } from 'vue'
import Dashboard from './components/Dashboard.vue'
import AgentWorkspace from './components/AgentWorkspace.vue'
import WarningCenter from './components/WarningCenter.vue'
import DocumentAssistant from './components/DocumentAssistant.vue'
import DataIntegration from './components/DataIntegration.vue'

const activeMenu = ref('5') // 默认停留你在截图里的页面，可自行更改

const handleMenuSelect = (index) => {
  activeMenu.value = index
}
</script>

<style scoped>
/* 强制整个应用的底色变为柔奶白 */
.hud-layout-container {
  height: 100vh;
  background-color: #F5EFEA;
  font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif;
  overflow: hidden;
}

/* ================= 侧边栏样式 ================= */
.hud-aside {
  background: #FFFFFF;
  border-right: 1px solid rgba(18, 46, 138, 0.1);
  display: flex;
  flex-direction: column;
  box-shadow: 2px 0 15px rgba(0, 0, 0, 0.03);
  position: relative;
  z-index: 10;
}

.logo-box {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  border-bottom: 1px solid rgba(18, 46, 138, 0.1);
  background: rgba(18, 46, 138, 0.02);
}

.logo-icon {
  width: 14px;
  height: 14px;
  background: #122E8A;
  transform: rotate(45deg);
}

.logo-text {
  font-size: 16px;
  font-weight: 900;
  color: #122E8A;
  letter-spacing: 1px;
}

/* 覆盖 Element Plus 菜单默认样式 */
.hud-menu {
  border-right: none !important;
  flex: 1;
  padding: 15px 10px;
}

:deep(.el-menu-item) {
  height: 44px;
  line-height: 44px;
  margin-bottom: 8px;
  border-radius: 4px;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 1px;
  transition: all 0.3s;
}

:deep(.el-menu-item:hover) {
  background-color: rgba(18, 46, 138, 0.05) !important;
  color: #122E8A !important;
}

:deep(.el-menu-item.is-active) {
  background-color: rgba(18, 46, 138, 0.08) !important;
  border-left: 4px solid #122E8A;
  font-weight: bold;
}

.aside-footer {
  padding: 20px 15px;
  border-top: 1px solid rgba(18, 46, 138, 0.1);
  font-family: 'JetBrains Mono', Consolas, monospace;
}

.sys-version {
  font-size: 11px;
  color: #666;
  margin-bottom: 5px;
  font-weight: bold;
}

.sys-status {
  font-size: 11px;
  color: #0F7E3B; /* 偏深的绿色更适合明亮主题 */
  display: flex;
  align-items: center;
  gap: 5px;
  font-weight: bold;
}

.blink-dot {
  width: 8px;
  height: 8px;
  background: #52C41A;
  border-radius: 50%;
  animation: blink 2s infinite;
}

/* ================= 顶部 Header 样式 ================= */
.hud-header {
  height: 60px;
  background: #FFFFFF;
  border-bottom: 1px solid rgba(18, 46, 138, 0.1);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 30px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.02);
  z-index: 9;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title-decorator {
  width: 4px;
  height: 18px;
  background: #122E8A;
  border-radius: 2px;
}

.main-title {
  font-size: 17px;
  font-weight: 900;
  color: #122E8A;
  letter-spacing: 1px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 20px;
}

.user-profile {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid transparent;
  transition: 0.3s;
}

.user-profile:hover {
  background: rgba(18, 46, 138, 0.05);
  border-color: rgba(18, 46, 138, 0.2);
}

.avatar-box {
  width: 30px;
  height: 30px;
  background: rgba(18, 46, 138, 0.1);
  border: 1px solid #122E8A;
  color: #122E8A;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 900;
  border-radius: 4px;
}

.user-name {
  font-size: 13px;
  font-weight: bold;
  color: #333;
}

/* ================= 主体区域 ================= */
.hud-main {
  padding: 0; /* 清除默认内边距 */
  background-color: transparent;
  position: relative;
  overflow: hidden;
}

.main-content-wrapper {
  width: 100%;
  height: 100%;
  position: relative;
}

/* Element Plus 下拉菜单强制适配明亮主题 */
:global(.el-popper.is-light) {
  background: #FFFFFF !important;
  border: 1px solid rgba(18, 46, 138, 0.15) !important;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.08) !important;
}
:global(.el-popper.is-light .el-popper__arrow::before) {
  background: #FFFFFF !important;
  border: 1px solid rgba(18, 46, 138, 0.15) !important;
}
:global(.el-dropdown-menu__item) {
  color: #333 !important;
  font-weight: 500;
}
:global(.el-dropdown-menu__item:hover) {
  background-color: rgba(18, 46, 138, 0.05) !important;
  color: #122E8A !important;
}

@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }
</style>