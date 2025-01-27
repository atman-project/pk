import { invoke } from "@tauri-apps/api/core";

let inputElement: HTMLInputElement | null;
let historyElement: HTMLElement | null;

// async function greet() {
//   if (greetMsgEl && greetInputEl) {
//     // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//     greetMsgEl.textContent = await invoke("greet", {
//       name: greetInputEl.value,
//     });
//   }
// }

window.addEventListener("DOMContentLoaded", () => {
  inputElement = document.getElementById("input")! as HTMLInputElement;
  historyElement = document.getElementById("history")!;

  inputElement.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      const command = inputElement!.value.trim();
      if (command) {
        const result = executeCommand(command);
        addHistory(command, result);
        inputElement!.value = "";
      }
    }
  });
});

function addHistory(command: string, result: string) {
  if (!historyElement) {
    return;
  }

  const commandElement = document.createElement("div");
  commandElement.className = "command";
  commandElement.textContent = `> ${command}`;

  const resultElement = document.createElement("div");
  resultElement.className = "result";
  resultElement.textContent = result;

  historyElement.prepend(commandElement);
  historyElement.prepend(resultElement);

  historyElement.scrollTop = historyElement.scrollHeight;
}

function executeCommand(command: string): string {
  switch (command) {
    case "help":
      return "Available commands: help, echo [text], clear";
    case "clear":
      if (historyElement) {
        historyElement.innerHTML = "";
      }
      return "";
    default:
      if (command.startsWith("echo ")) {
        return command.slice(5);
      }
      return "Unknown command";
  }
}
