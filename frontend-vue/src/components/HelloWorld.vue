<script setup>
import { ref } from 'vue'
import viteLogo from '../assets/vite.svg'
import heroImg from '../assets/hero.png'
import vueLogo from '../assets/vue.svg'

const count = ref(0)
</script>

<template>
  <div class="hello-world-container">
    <section id="center">
      <div class="hero">
        <img :src="heroImg" class="base" width="170" height="179" alt="" />
        <img :src="vueLogo" class="framework" alt="Vue logo" />
        <img :src="viteLogo" class="vite" alt="Vite logo" />
      </div>
      <div class="text-content">
        <h1>Get started</h1>
        <p>Edit <code>src/App.vue</code> and save to test <code>HMR</code></p>
      </div>
      <button type="button" class="counter-btn" @click="count++">
        Count is {{ count }}
      </button>
    </section>

    <div class="ticks"></div>

    <section id="next-steps">
      <div id="docs" class="card">
        <svg class="icon" role="presentation" aria-hidden="true">
          <use href="/icons.svg#documentation-icon"></use>
        </svg>
        <h2>Documentation</h2>
        <p>Your questions, answered</p>
        <ul>
          <li>
            <a href="https://vite.dev/" target="_blank">
              <img class="logo" :src="viteLogo" alt="" />
              Explore Vite
            </a>
          </li>
          <li>
            <a href="https://vuejs.org/" target="_blank">
              <img class="button-icon" :src="vueLogo" alt="" />
              Learn more
            </a>
          </li>
        </ul>
      </div>
      <div id="social" class="card">
        <svg class="icon" role="presentation" aria-hidden="true">
          <use href="/icons.svg#social-icon"></use>
        </svg>
        <h2>Connect with us</h2>
        <p>Join the Vite community</p>
        <ul>
          <li>
            <a href="https://github.com/vitejs/vite" target="_blank">
              <svg class="button-icon" role="presentation" aria-hidden="true">
                <use href="/icons.svg#github-icon"></use>
              </svg>
              GitHub
            </a>
          </li>
          <li>
            <a href="https://chat.vite.dev/" target="_blank">
              <svg class="button-icon" role="presentation" aria-hidden="true">
                <use href="/icons.svg#discord-icon"></use>
              </svg>
              Discord
            </a>
          </li>
          <li>
            <a href="https://x.com/vite_js" target="_blank">
              <svg class="button-icon" role="presentation" aria-hidden="true">
                <use href="/icons.svg#x-icon"></use>
              </svg>
              X.com
            </a>
          </li>
          <li>
            <a href="https://bsky.app/profile/vite.dev" target="_blank">
              <svg class="button-icon" role="presentation" aria-hidden="true">
                <use href="/icons.svg#bluesky-icon"></use>
              </svg>
              Bluesky
            </a>
          </li>
        </ul>
      </div>
    </section>

    <div class="ticks"></div>
    <section id="spacer"></section>
  </div>
</template>

<style scoped>
.hello-world-container {
  background-color: #F5EFEA; /* 柔奶白全局背景 */
  color: #333333;
  font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif;
  padding: 60px 20px;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
}

#center {
  text-align: center;
  margin-bottom: 40px;
}

.hero {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 20px;
  margin-bottom: 30px;
}

.text-content h1 {
  color: #122E8A; /* 深海蓝主题色 */
  font-size: 38px;
  margin-bottom: 12px;
  font-weight: 900;
  letter-spacing: 1px;
}

.text-content p {
  color: #666;
  font-size: 16px;
}

code {
  background: #FFFFFF;
  border: 1px solid rgba(18, 46, 138, 0.2);
  color: #122E8A;
  padding: 3px 8px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', monospace;
  font-weight: bold;
}

.counter-btn {
  margin-top: 25px;
  background-color: #122E8A;
  color: #FFFFFF;
  border: none;
  padding: 12px 28px;
  font-size: 16px;
  font-weight: bold;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s ease;
  box-shadow: 0 4px 15px rgba(18, 46, 138, 0.15);
}

.counter-btn:hover {
  background-color: #0D226A;
  box-shadow: 0 6px 20px rgba(18, 46, 138, 0.25);
  transform: translateY(-2px);
}

#next-steps {
  display: flex;
  gap: 30px;
  max-width: 850px;
  width: 100%;
  justify-content: center;
  flex-wrap: wrap;
}

.card {
  background: #FFFFFF;
  border: 1px solid rgba(18, 46, 138, 0.15);
  border-radius: 8px;
  padding: 30px;
  flex: 1;
  min-width: 320px;
  box-shadow: 0 4px 20px rgba(18, 46, 138, 0.05);
  transition: transform 0.3s ease, box-shadow 0.3s ease;
}

.card:hover {
  transform: translateY(-5px);
  box-shadow: 0 8px 30px rgba(18, 46, 138, 0.1);
}

.card h2 {
  color: #122E8A;
  font-size: 22px;
  margin: 15px 0 8px;
  font-weight: bold;
}

.card p {
  color: #666;
  font-size: 14px;
  margin-bottom: 24px;
}

.icon {
  width: 36px;
  height: 36px;
  color: #122E8A;
}

ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

li a {
  display: flex;
  align-items: center;
  gap: 12px;
  text-decoration: none;
  color: #333;
  font-weight: 600;
  padding: 12px 16px;
  border-radius: 6px;
  background: #F9F9F9;
  border: 1px solid rgba(18, 46, 138, 0.05);
  transition: all 0.2s ease;
}

li a:hover {
  background: rgba(18, 46, 138, 0.05);
  border-color: #122E8A;
  color: #122E8A;
}

.button-icon, .logo {
  width: 22px;
  height: 22px;
}

.ticks {
  width: 100%;
  height: 1px;
  background: rgba(18, 46, 138, 0.15);
  margin: 40px 0;
  max-width: 850px;
}
</style>