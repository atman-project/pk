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

  startBackgroundTask();
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
      try {
        return await invoke("execute_command", { command: command });
      } catch (error) {
        const errorStr = `Error: ${String(error)}`;
        console.error(errorStr);
        return errorStr;
      }
  }
}

async function startBackgroundTask() {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  while (true) {
    try {
      console.log("Waiting for a next background output...");
      const msg: string = await invoke("next_bg_output");
      console.log("Received a next background output:", msg);
      addHistory("[Background output]", msg);
    } catch (error) {
      addHistory("[Background error]", `Error: ${String(error)}`);
    }
  }
}
