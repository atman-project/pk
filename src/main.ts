import { invoke } from "@tauri-apps/api/core";

let inputElement: HTMLInputElement | null;
let historyElement: HTMLElement | null;

window.addEventListener("DOMContentLoaded", () => {
  inputElement = document.getElementById("input")! as HTMLInputElement;
  historyElement = document.getElementById("history")!;

  inputElement.addEventListener("keydown", async (event) => {
    if (event.key === "Enter") {
      const command = inputElement!.value.trim();
      if (command) {
        const result = await executeCommand(command);
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

async function executeCommand(command: string): Promise<string> {
  switch (command) {
    case "help":
      return "Available commands: help, clear, or [any_text]";
    case "clear":
      if (historyElement) {
        historyElement.innerHTML = "";
      }
      return "";
    default:
      return await invoke("execute_command", { command: command });
  }
}
