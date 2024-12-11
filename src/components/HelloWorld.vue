<template>
  <div>
    <h1>Select SD Card</h1>
    <select v-model="selectedDevice">
      <option v-for="device in devices" :key="device" :value="device">
        {{ device[0] + device[1] }}
      </option>
    </select>

    <button @click="uploadFiles">Upload</button>

    <div v-if="progress >= 0">
      <progress :value="progress" max="100">{{ progress }}%</progress>
    </div>
  </div>
</template>

<script>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api";

export default {
  setup() {
    const devices = ref([]);
    const selectedDevice = ref(null);
    const progress = ref(-1); // -1 means no progress yet

    const loadDevices = async () => {
      try {
        devices.value = await invoke("list_devices");
      } catch (error) {
        console.error("Failed to load devices:", error);
      }
    };

    const uploadFiles = async () => {
      if (!selectedDevice.value) {
        alert("Please select a device!");
        return;
      }

      const dest = "C:/Users/Jayce Olson/Desktop/uploader_test"; // Adjust destination as needed
      try {
        await invoke("copy_dir", {
          src: selectedDevice.value[0],
          dest,
        });
        alert("Files uploaded successfully!");
      } catch (error) {
        console.error("Failed to upload files:", error);
        alert("Error during file upload");
      }
    };

    onMounted(loadDevices);

    return { devices, selectedDevice, uploadFiles, progress };
  },
};
</script>

<style src="./HelloWorld.css"></style>
