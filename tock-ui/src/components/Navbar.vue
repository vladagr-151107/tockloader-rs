<script setup>
import { ref } from 'vue';

const tabs = [
  'CONNECTED DEVICES',
  'APP MANAGEMENT',
  'TERMINAL',
  
];

const activeTab = ref('CONNECTED DEVICES');
</script>

<template>
 
  <nav class="flex items-center justify-center px-6 py-3 bg-[#424242] font-sans">
    
    <div class="flex space-x-2 text-xs font-bold tracking-wider">
      <button
        v-for="tab in tabs"
        :key="tab"
        @click="activeTab = tab"
        :class="[
          'px-4 py-2 transition-all duration-200 ease-in-out rounded-full',
          activeTab === tab 
            ? 'bg-[#555555] text-white' 
            : 'text-gray-300 hover:text-white hover:bg-[#4a4a4a]'
        ]"
      >
        {{ tab }}
      </button>
    </div>
    
  </nav>
</template>