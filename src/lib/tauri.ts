import { invoke } from "@tauri-apps/api/core";

export type ZuzuApplication =
  | "Visual Studio Code"
  | "Google Chrome"
  | "Microsoft Edge"
  | "Windows File Explorer"
  | "Notepad";

export const listFiles = (): Promise<string[]> =>
  invoke<string[]>("list_files");

export const readTextFile = (filename: string): Promise<string> =>
  invoke<string>("read_text_file", { filename });

export const createTextFile = (
  filename: string,
  content: string,
): Promise<void> =>
  invoke("create_text_file", { filename, content });

export const writeTextFile = (
  filename: string,
  content: string,
): Promise<void> =>
  invoke("write_text_file", { filename, content });

export const openFile = (filename: string): Promise<void> =>
  invoke("open_file", { filename });

export const openFolder = (): Promise<void> =>
  invoke("open_folder");

export const openApplication = (
  application: ZuzuApplication,
): Promise<void> =>
  invoke("open_application", { application });
