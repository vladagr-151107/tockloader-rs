<script setup>
import Navbar from '../components/Navbar.vue';
</script>

<template>
  
  <div class="flex flex-col min-h-screen">

    <Navbar />

    <div class="flex flex-col items-center justify-center flex-1 w-full gap-8 text-center text-gray-200">

        <h1 class="text-5xl">No board detected.<br/> Click here to set up your board</h1>

        <button class="px-8 py-3 text-lg shadow-lg font-bold text-white transition-colors duration-200 bg-[#4C4C4C] rounded-lg hover:bg-[#605F5F]">
          Choose Board
        </button>
        
    </div>
    <Card></Card>
    
  </div>
</template>