<template>
  <div class="pet-container" data-tauri-drag-region>
    <!-- 终极抠图魔法：通过数学矩阵，精准把白色像素的透明度降为 0 -->
    <svg width="0" height="0" style="position: absolute;">
      <filter id="magic-wand" x="0%" y="0%" width="100%" height="100%">
        <!-- 1. 提取颜色并把纯白色变为透明，但会错误地把周围透明区域变为黑色 -->
        <feColorMatrix in="SourceGraphic" type="matrix" values="
          1 0 0 0 0
          0 1 0 0 0
          0 0 1 0 0
          -5 -5 -5 0 14.5
        " result="color_mask" />
        <!-- 2. 关键修复：用原图的透明度进行交叉裁剪，完美去除黑框 -->
        <feComposite in="color_mask" in2="SourceGraphic" operator="in" />
      </filter>
    </svg>
    
    <!-- 放弃 vue 层的事件绑定，纯粹依赖 Tauri 的拖拽区域 -->
    <div class="pet-area" data-tauri-drag-region @mousedown="startDrag">
      <!-- 恢复包裹层，用于精准定位保护层 -->
      <div class="pet-image-wrapper" data-tauri-drag-region>
        <!-- 精准定位：只垫在右下角白熊的肚子下面 -->
        <div class="pet-body-protector" data-tauri-drag-region></div>
        <div class="pet-image-container" data-tauri-drag-region>
          <img :src="petImage" class="pet-gif" alt="pet" draggable="false" />
        </div>
      </div>
      <div class="pet-info" v-if="showTasks" data-tauri-drag-region>
        <p class="pet-name">🐻 {{ petName }}</p>
        <p class="pet-mood">{{ moodText }}</p>
      </div>
    </div>
    <!-- ... 后面的气泡代码保持不变 ... -->
    <div class="speech-bubble" v-if="showTasks">
      <h2>📝 今日任务</h2>
      <div class="input-group">
        <input 
          v-model="newTask" 
          @keyup.enter="addTask"
          placeholder="添加新任务..." 
        />
        <button @click="addTask">+</button>
      </div>
      <ul class="task-list">
        <li v-for="(task, index) in tasks" :key="index" :class="{ completed: task.completed }">
          <div class="task-content" @click="toggleTask(index)">
            <input type="checkbox" :checked="task.completed" readonly />
            <span>{{ task.text }}</span>
          </div>
          <button class="delete-btn" @click="deleteTask(index)">×</button>
        </li>
      </ul>
      <p class="progress">完成: {{ completedCount }}/{{ tasks.length }}</p>
      <div class="autostart-row" v-if="!isMobile">
  <label class="autostart-label">
    <input type="checkbox" :checked="autoStartEnabled" @change="onAutoStartChange" />
    开机自启动
  </label>
</div>
      <button class="close-panel" @click="showTasks = false">收起</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import { type } from '@tauri-apps/plugin-os'
import petImg from './assets/122.gif'

// 检测是否为移动端 (android 或 ios)
const isMobile = ref(false)
const checkPlatform = async () => {
  try {
    const platformName = type()
    isMobile.value = platformName === 'android' || platformName === 'ios'
    console.log('当前平台:', platformName, '是否移动端:', isMobile.value)
  } catch (e) {
    console.error('获取平台失败:', e)
  }
}

interface Task {
  text: string
  completed: boolean
}

const petName = ref('一二')
const petState = ref<'idle' | 'happy' | 'sad'>('idle')

const petImage = computed(() => {
  return petImg
})
const newTask = ref('')
const tasks = ref<Task[]>([])
const showTasks = ref(false)
const autoStartEnabled = ref(false)
let dragActive = false
let dragged = false
let dragStartX = 0
let dragStartY = 0
let dragWindowX = 0
let dragWindowY = 0
const dragThreshold = 4
const rightBottomMargin = 12

const completedCount = computed(() => tasks.value.filter(t => t.completed).length)
const moodText = computed(() => {
  if (completedCount.value === 0) return '有点无聊...'
  if (completedCount.value === tasks.value.length) return '太棒了！🎉'
  if (completedCount.value > tasks.value.length / 2) return '很开心！😊'
  return '继续加油！💪'
})

// 添加新任务的函数
const addTask = () => {
  // 检查输入是否为空（去除空格后）
  if (newTask.value.trim()) {
    // 将新任务对象添加到 tasks 数组中
    // text: 任务内容, completed: 是否完成（初始为 false）
    tasks.value.push({ 
      text: newTask.value.trim(), 
      completed: false 
    })
    
    // 清空输入框，方便下次输入
    newTask.value = ''
    
    // 任务变动后，更新宠物的心情状态（比如任务多了会变难过，全清会变开心）
    updatePetState()
    
    // 将更新后的任务列表保存到本地存储 localStorage，防止刷新后丢失
    saveTasks()
  }
}

// 切换任务完成状态的函数
const toggleTask = (index: number) => {
  // 1. 找到对应的任务，并将其完成状态取反（true 变 false，false 变 true）
  tasks.value[index].completed = !tasks.value[index].completed
  
  // 2. 状态改变后，更新宠物的心情（比如任务完成了，宠物会变开心）
  updatePetState()
  
  // 3. 将最新的任务状态同步到 Rust 后端（存入本地 JSON 文件）
  saveTasks()
}

// 删除任务的函数
const deleteTask = (index: number) => {
  // 1. 从 tasks 数组中移除指定位置的任务
  tasks.value.splice(index, 1)
  
  // 2. 同样需要更新心情和保存数据
  updatePetState()
  saveTasks()
}

const updatePetState = () => {
  const rate = completedCount.value / Math.max(tasks.value.length, 1)
  if (rate === 1 && tasks.value.length > 0) {
    petState.value = 'happy'
  } else if (rate === 0 && tasks.value.length > 2) {
    petState.value = 'sad'
  } else {
    petState.value = 'idle'
  }
}

// 调用 Rust 后端保存任务
const saveTasks = async () => {
  try {
    // 调用我们在 main.rs 中注册的 save_tasks_rust 命令
    await invoke('save_tasks_rust', { tasks: tasks.value })
    console.log('任务已保存到 Rust 后端')
  } catch (e) {
    console.error('保存任务失败:', e)
  }
}

// 调用 Rust 后端读取任务
const loadTasks = async () => {
  try {
    // 调用我们在 main.rs 中注册的 load_tasks_rust 命令
    const savedTasks = await invoke<Task[]>('load_tasks_rust')
    tasks.value = savedTasks
    console.log('任务已从 Rust 后端加载')
  } catch (e) {
    console.error('加载任务失败:', e)
  }
}

const handleDragMove = async (e: MouseEvent) => {
  if (!dragActive) return
  const dx = e.screenX - dragStartX
  const dy = e.screenY - dragStartY
  if (!dragged && (Math.abs(dx) > dragThreshold || Math.abs(dy) > dragThreshold)) {
    dragged = true
  }
  const targetX = Math.floor(dragWindowX + dx)
  const targetY = Math.floor(dragWindowY + dy)
  try {
    await invoke('move_window', { x: targetX, y: targetY })
  } catch (e) {
    console.error('移动窗口失败:', e)
  }
}

const handleDragEnd = (e: MouseEvent) => {
  if (!dragActive) return
  window.removeEventListener('mousemove', handleDragMove)
  window.removeEventListener('mouseup', handleDragEnd)
  dragActive = false
  if (!dragged && e.button === 0) {
    showTasks.value = !showTasks.value
  }
}

const startDrag = (e: MouseEvent) => {
  if (e.button !== 0) return
  dragActive = true
  dragged = false
  dragStartX = e.screenX
  dragStartY = e.screenY
  dragWindowX = window.screenX
  dragWindowY = window.screenY
  window.addEventListener('mousemove', handleDragMove)
  window.addEventListener('mouseup', handleDragEnd)
}

const moveToRightBottom = async () => {
  if (isMobile.value) return // 移动端跳过窗口移动
  const targetX = Math.floor(window.screen.availWidth - window.innerWidth - rightBottomMargin)
  const targetY = Math.floor(window.screen.availHeight - window.innerHeight - rightBottomMargin)
  try {
    await invoke('move_window', { x: targetX, y: targetY })
  } catch (e) {
    console.error('移动窗口失败:', e)
  }
}

const loadAutoStart = async () => {
  if (isMobile.value) return // 移动端跳过自启动逻辑
  try {
    autoStartEnabled.value = await isEnabled()
  } catch (e) {
    console.error('获取开机自启动状态失败:', e)
  }
}

const onAutoStartChange = async (e: Event) => {
  if (isMobile.value) return
  const checked = (e.target as HTMLInputElement).checked
  try {
    if (checked) {
      await enable()
    } else {
      await disable()
    }
    autoStartEnabled.value = checked
  } catch (err) {
    autoStartEnabled.value = !checked
    console.error('设置开机自启动失败:', err)
  }
}

// 随机移动逻辑
const startRoaming = async () => {
  if (isMobile.value) return // 移动端跳过巡逻
  const move = async () => {
    // 只有在没打开任务面板时才移动
    if (!showTasks.value && !dragActive) {
      // 使用浏览器全局的 screen 属性，而不是 Tauri 窗口
      const targetX = Math.floor(Math.random() * (window.screen.availWidth - 200))
      const targetY = Math.floor(Math.random() * (window.screen.availHeight - 200))
      
      try {
        await invoke('move_window', { x: targetX, y: targetY })
      } catch (e) {
        console.error('移动巡逻失败:', e)
      }
    }
    
    // 每隔 5-10 秒移动一次
    const nextMove = 5000 + Math.random() * 5000
    setTimeout(move, nextMove)
  }
  
  move()
}

onMounted(async () => {
  await checkPlatform()
  await loadTasks()
  updatePetState()
  await moveToRightBottom()
  await loadAutoStart()
  startRoaming() // 启动随机巡逻
  
  // 监听全局右键，防止默认菜单
  window.addEventListener('contextmenu', (e) => e.preventDefault())
})
</script>

<style scoped>
#app {
  background: transparent !important;
}

.pet-container {
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: row; /* 横向排列 */
  align-items: center;
  justify-content: flex-end; /* 宠物靠右 */
  padding-right: 40px;
  position: relative;
  background: transparent !important;
  overflow: hidden;
}

/* 移动端适配样式 */
@media (max-width: 600px) {
  .pet-container {
    justify-content: center; /* 手机上居中 */
    padding-right: 0;
  }
  
  .task-panel {
    width: 90vw !important; /* 手机上任务面板更宽 */
    right: 5vw !important;
    bottom: 160px !important;
  }
  
  .pet-image-wrapper {
    width: 120px !important; /* 手机上小一点 */
    height: 120px !important;
  }
}

/* ... 替换之前的复杂样式，改用极致简单的物理裁切 ... */
.pet-area {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  cursor: grab;
  transition: transform 0.2s;
  z-index: 10;
  /* 防止选中文字或图片影响拖拽 */
  user-select: none;
  -webkit-user-select: none;
  -webkit-user-drag: none;
}

.pet-area:active {
  cursor: grabbing;
}

.pet-image-wrapper {
  position: relative;
  width: 150px;
  height: 150px;
}

/* 精准定位：只保护右下角的白熊，绝不干扰棕熊 */
.pet-body-protector {
  position: absolute;
  width: 50px;       /* 进一步缩小，防止光晕溢出到外面 */
  height: 30px;      /* 缩小高度 */
  background: white;
  border-radius: 50%;
  bottom: 20px;      /* 稍微往下移 */
  right: 30px;       /* 稍微往左移，精确对准白熊肚子 */
  z-index: 0;
  filter: blur(2px); /* 减小模糊半径，防止泛白 */
}

.pet-image-container {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  transition: transform 0.3s;
}

.pet-gif {
  width: 100%;
  height: 100%;
  object-fit: contain;
  animation: pet-breathing 3s ease-in-out infinite;
  /* 把滤镜和 clip-path 移到图片本身，这样不会封死外层 div 的拖拽事件 */
  filter: url(#magic-wand);
  clip-path: inset(3px);
}

/* 简单的呼吸缩放，不干扰 GIF 自带的走路动作 */
@keyframes pet-breathing {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.05); }
}

.pet-area:hover .pet-image-container {
  transform: scale(1.1);
}

.pet-name {
  font-size: 14px;
  font-weight: bold;
  color: #333;
  cursor: pointer;
  padding: 4px 8px;
  background: rgba(255,255,255,0.8);
  border-radius: 20px;
  margin-top: 8px;
  display: inline-block;
  user-select: none;
}

.pet-name:hover {
  background: white;
  color: #667eea;
}

.pet-mood {
  font-size: 11px;
  color: #666;
  margin-top: 2px;
}

/* 气泡对话框样式 */
.speech-bubble {
  position: absolute;
  right: 160px; /* 在宠物左侧 */
  top: 50%;
  transform: translateY(-50%);
  width: 280px; /* 调大宽度 */
  background: white;
  border-radius: 16px;
  padding: 16px;
  box-shadow: 0 8px 30px rgba(0,0,0,0.2);
  z-index: 100;
  animation: slideIn 0.3s ease;
}

/* 气泡小尾巴：指向右侧的宠物 */
.speech-bubble::after {
  content: '';
  position: absolute;
  right: -10px;
  top: 50%;
  transform: translateY(-50%);
  border-top: 10px solid transparent;
  border-bottom: 10px solid transparent;
  border-left: 10px solid white;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateY(-50%) translateX(-20px); }
  to { opacity: 1; transform: translateY(-50%) translateX(0); }
}

.speech-bubble h2 {
  font-size: 18px; /* 字体调大 */
  margin-bottom: 12px;
  color: #333;
}

.input-group {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
}

.input-group input {
  flex: 1;
  padding: 4px 8px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 11px;
}

.input-group button {
  background: #667eea;
  color: white;
  border: none;
  border-radius: 6px;
  padding: 0 8px;
  cursor: pointer;
  font-size: 11px;
}

.task-list {
  list-style: none;
  max-height: 180px; /* 调大列表区域 */
  overflow-y: auto;
}

.task-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  padding: 8px;
  background: #f8f9fa;
  margin-bottom: 6px;
  border-radius: 8px;
  transition: all 0.2s;
}

.task-content {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  flex: 1;
}

.task-content input {
  cursor: pointer;
}

.task-list li.completed {
  background: #f1f3f5;
}

.task-list li.completed span {
  text-decoration: line-through;
  color: #adb5bd;
}

.delete-btn {
  background: #ff6b6b;
  color: white;
  border: none;
  border-radius: 4px;
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.progress {
  font-size: 10px;
  color: #666;
  text-align: center;
  margin: 8px 0;
}

.autostart-row {
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 6px 0 10px;
}

.autostart-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #333;
  cursor: pointer;
}

.autostart-label input {
  width: 12px;
  height: 12px;
}

.close-panel {
  width: 100%;
  padding: 4px;
  background: #eee;
  border: none;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
}
</style>
