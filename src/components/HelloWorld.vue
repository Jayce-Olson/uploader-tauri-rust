<template>
  <div id="app">
    <h1>Value: {{ value }}</h1>
    <button @click="addX">Add X</button>
  </div>
</template>

<script>
import { ref } from "vue";
import { invoke } from "@tauri-apps/api";
// const invoke = window.__TAURI__.invoke;

export default {
  setup() {
    const value = ref(0); // Because this is Vue Composistion, I can use ref() to create a reactive reference to this value (Allows Vue to track changes and automatically update the DOM)
    const addX = async () => {
      // Async because I am making a call to the Rust backend
      if (!window.__TAURI__) {
        console.error("Tauri API is not available. Check your configuration.");
        return;
      }

      try {
        // Call the Rust command
        value.value = await invoke("add_x", {
          currentValue: value.value,
        });
      } catch (error) {
        console.error("Failed invoke addX from backend: ", error);
      }
    };

    return { value, addX }; // I am returning value and addX in order to make them accesible in the template, this will allow me to display value, and call addX() from the template (HTML)
  },
};
</script>

<style scoped>
#app {
  text-align: center;
  margin-top: 50px;
}
button {
  padding: 10px 20px;
  font-size: 16px;
}
</style>
