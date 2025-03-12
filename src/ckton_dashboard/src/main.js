import { createPinia } from 'pinia';
import { createApp } from 'vue';
import './index.css';
import App from './App.vue';
import { toast } from 'vue3-toastify';
import 'vue3-toastify/dist/index.css';

createApp(App).use(createPinia()).mount('#app');
