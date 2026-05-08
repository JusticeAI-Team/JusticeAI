<template>
  <div class="globe-transparent-container">
    <div ref="hostRef" class="globe-host"></div>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import * as THREE from 'three'

const hostRef = ref(null)

let renderer = null
let scene = null
let camera = null
let frameId = 0
let globe = null
let atmosphere = null
let ring = null
let resizeObserver = null
let pointerX = 0
let pointerY = 0

// 创建背景数据粒子（原星空，现改为深蓝色的数据悬浮点）
const createStars = () => {
  const geometry = new THREE.BufferGeometry()
  const count = 900
  const positions = new Float32Array(count * 3)

  for (let index = 0; index < count; index += 1) {
    const radius = 10 + Math.random() * 20
    const theta = Math.random() * Math.PI * 2
    const phi = Math.acos(2 * Math.random() - 1)
    positions[index * 3] = radius * Math.sin(phi) * Math.cos(theta)
    positions[index * 3 + 1] = radius * Math.sin(phi) * Math.sin(theta)
    positions[index * 3 + 2] = radius * Math.cos(phi)
  }

  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
  // 粒子颜色改为深海蓝，降低透明度，作为白底的点缀
  const material = new THREE.PointsMaterial({
    color: 0x122E8A, 
    size: 0.035,
    transparent: true,
    opacity: 0.3,
    depthWrite: false
  })

  const stars = new THREE.Points(geometry, material)
  scene.add(stars)
}

const handlePointerMove = (event) => {
  const host = hostRef.value
  if (!host) return
  const rect = host.getBoundingClientRect()
  const x = ((event.clientX - rect.left) / rect.width) * 2 - 1
  const y = -(((event.clientY - rect.top) / rect.height) * 2 - 1)
  pointerX = x * 0.65
  pointerY = y * 0.35
}

const handlePointerLeave = () => {
  pointerX = 0
  pointerY = 0
}

const animate = () => {
  frameId = window.requestAnimationFrame(animate)

  if (globe) {
    globe.rotation.y += 0.003
    globe.rotation.x += 0.0004
    globe.rotation.z = THREE.MathUtils.lerp(globe.rotation.z, pointerX * 0.18, 0.04)
    globe.rotation.x = THREE.MathUtils.lerp(globe.rotation.x, pointerY * 0.14, 0.04)
  }
  if (atmosphere) atmosphere.rotation.y -= 0.0015
  if (ring) {
    ring.rotation.z += 0.0014
    ring.rotation.x = Math.PI / 2.4 + pointerY * 0.05
  }
  if (renderer && scene && camera) renderer.render(scene, camera)
}

const buildScene = () => {
  const host = hostRef.value
  if (!host) return

  scene = new THREE.Scene()
  camera = new THREE.PerspectiveCamera(42, host.clientWidth / host.clientHeight, 0.1, 100)
  camera.position.set(0, 0.15, 4.2)

  // 必须保持透明背景，以便透出父组件的柔奶白底色
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true })
  renderer.setSize(host.clientWidth, host.clientHeight)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.setClearColor(0x000000, 0) 
  host.appendChild(renderer.domElement)

  // 1. 地球本体：深海蓝哑光质感
  const globeGeometry = new THREE.SphereGeometry(1.15, 72, 72)
  const globeMaterial = new THREE.MeshStandardMaterial({
    color: 0x122E8A,    // 主色：深海蓝
    metalness: 0.15,    // 降低金属感，使其更像严谨的模型
    roughness: 0.7,     // 增加粗糙度，避免反光过于锐利
    emissive: 0x0A1845, // 微弱的深蓝自发光，防止暗部死黑
    emissiveIntensity: 0.5
  })
  globe = new THREE.Mesh(globeGeometry, globeMaterial)
  scene.add(globe)

  // 2. 数据网格层（原大气层）：深蓝色线框
  const atmosphereGeometry = new THREE.SphereGeometry(1.22, 72, 72)
  const atmosphereMaterial = new THREE.MeshBasicMaterial({
    color: 0x122E8A, 
    transparent: true,
    opacity: 0.15,
    wireframe: true
  })
  atmosphere = new THREE.Mesh(atmosphereGeometry, atmosphereMaterial)
  scene.add(atmosphere)

  // 3. 卫星轨道环：深蓝色半透明
  const ringGeometry = new THREE.TorusGeometry(1.65, 0.03, 16, 180)
  const ringMaterial = new THREE.MeshBasicMaterial({
    color: 0x122E8A,
    transparent: true,
    opacity: 0.25
  })
  ring = new THREE.Mesh(ringGeometry, ringMaterial)
  ring.rotation.x = Math.PI / 2.4
  scene.add(ring)

  // 4. 灯光调整：适配浅色背景的补光
  const ambient = new THREE.AmbientLight(0xffffff, 1.8) // 增强环境光
  const directional = new THREE.DirectionalLight(0xffffff, 1.2)
  directional.position.set(4, 3, 5)
  // 边缘光改为柔和的蓝色，去除刺眼的青色
  const rim = new THREE.PointLight(0x4A90E2, 1.5, 20) 
  rim.position.set(-3, -1, 3)
  scene.add(ambient, directional, rim)

  createStars()

  host.addEventListener('pointermove', handlePointerMove)
  host.addEventListener('pointerleave', handlePointerLeave)

  resizeObserver = new ResizeObserver(() => {
    if (!host || !renderer || !camera) return
    camera.aspect = host.clientWidth / host.clientHeight
    camera.updateProjectionMatrix()
    renderer.setSize(host.clientWidth, host.clientHeight)
  })
  resizeObserver.observe(host)

  animate()
}

onMounted(() => {
  buildScene()
})

onBeforeUnmount(() => {
  if (frameId) window.cancelAnimationFrame(frameId)
  const host = hostRef.value
  if (host) {
    host.removeEventListener('pointermove', handlePointerMove)
    host.removeEventListener('pointerleave', handlePointerLeave)
  }
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  if (scene) {
    scene.traverse((object) => {
      if (object.geometry) object.geometry.dispose()
      if (object.material) {
        if (Array.isArray(object.material)) {
          object.material.forEach((m) => m.dispose())
        } else {
          object.material.dispose()
        }
      }
    })
  }
  if (renderer) {
    renderer.dispose()
    renderer.forceContextLoss()
    renderer.domElement?.remove()
  }
})
</script>

<style scoped>
.globe-transparent-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent; 
}

.globe-host {
  width: 100%;
  height: 100%;
}
</style>