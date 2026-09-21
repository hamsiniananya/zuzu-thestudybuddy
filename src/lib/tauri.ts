import { invoke } from "@tauri-apps/api/core";

export type ZuzuApplication =
  | "vscode"
  | "chrome"
  | "edge"
  | "whatsapp"
  | "explorer"
  | "notepad"
  | "powershell"
  | "spotify"
  | "calculator"
  | "settings";

export type PendingWhatsAppMessage = {
  alias: string;
  recipient: string;
  message: string;
};

type ResolvedWhatsAppContact = {
  name: string;
  phone: string;
};

type ActionRequest =
  | { action: "list_files" }
  | { action: "read_text_file"; filename: string }
  | { action: "create_text_file"; filename: string; content: string }
  | { action: "write_text_file"; filename: string; content: string }
  | { action: "open_file"; filename: string }
  | { action: "open_folder" }
  | { action: "open_application"; application: ZuzuApplication }
  | { action: "send_whatsapp_message"; recipient: string; message: string };

type ZuzuResponse = {
  type: "message";
  content: string;
} | {
  type: "action";
  action: ActionRequest;
};

export type RoutedZuzuResponse =
  | string
  | {
      kind: "whatsapp_confirmation";
      alias: string;
      recipient: string;
      message: string;
    };

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

export const resolveWhatsAppContact = (
  recipient: string,
): Promise<ResolvedWhatsAppContact> =>
  invoke<ResolvedWhatsAppContact>("resolve_whatsapp_contact_command", {
    recipient,
  });

export const sendWhatsAppMessage = (
  recipient: string,
  message: string,
): Promise<string> =>
  invoke<string>("send_whatsapp_message", { recipient, message });

const isNonEmptyString = (value: unknown): value is string =>
  typeof value === "string" && value.trim().length > 0;

const isSafeFilename = (value: unknown): value is string =>
  isNonEmptyString(value) &&
  value !== "." &&
  value !== ".." &&
  !value.includes("/") &&
  !value.includes("\\") &&
  !value.includes("\0");

const isApplication = (value: unknown): value is ZuzuApplication =>
  value === "vscode" ||
  value === "chrome" ||
  value === "edge" ||
  value === "whatsapp" ||
  value === "explorer" ||
  value === "notepad" ||
  value === "powershell" ||
  value === "spotify" ||
  value === "calculator" ||
  value === "settings";

const parseResponse = (response: string): ZuzuResponse | null => {
  try {
    const parsed: unknown = JSON.parse(response);

    if (
      typeof parsed !== "object" ||
      parsed === null ||
      !("type" in parsed)
    ) {
      return null;
    }

    if (
      parsed.type === "message" &&
      "content" in parsed &&
      isNonEmptyString(parsed.content)
    ) {
      return { type: "message", content: parsed.content };
    }

    if (
      parsed.type !== "action" ||
      !("action" in parsed) ||
      typeof parsed.action !== "object" ||
      parsed.action === null ||
      !("action" in parsed.action)
    ) {
      return null;
    }

    const action = parsed.action;
    if (action.action === "list_files" || action.action === "open_folder") {
      return { type: "action", action: { action: action.action } };
    }

    if (
      (action.action === "read_text_file" ||
        action.action === "open_file") &&
      "filename" in action &&
      isSafeFilename(action.filename)
    ) {
      return {
        type: "action",
        action: { action: action.action, filename: action.filename },
      };
    }

    if (
      (action.action === "create_text_file" ||
        action.action === "write_text_file") &&
      "filename" in action &&
      "content" in action &&
      isSafeFilename(action.filename) &&
      typeof action.content === "string"
    ) {
      return {
        type: "action",
        action: {
          action: action.action,
          filename: action.filename,
          content: action.content,
        },
      };
    }

    if (
      action.action === "open_application" &&
      "application" in action &&
      isApplication(action.application)
    ) {
      return {
        type: "action",
        action: { action: "open_application", application: action.application },
      };
    }

    if (
      action.action === "send_whatsapp_message" &&
      "recipient" in action &&
      "message" in action &&
      isNonEmptyString(action.recipient) &&
      isNonEmptyString(action.message) &&
      action.recipient.length <= 100 &&
      action.message.length <= 4000
    ) {
      return {
        type: "action",
        action: {
          action: "send_whatsapp_message",
          recipient: action.recipient,
          message: action.message,
        },
      };
    }
  } catch {
    return null;
  }

  return null;
};

export const routeZuzuResponse = async (
  response: string,
): Promise<RoutedZuzuResponse> => {
  const parsed = parseResponse(response);
  if (!parsed) return response;
  if (parsed.type === "message") return parsed.content;

  switch (parsed.action.action) {
    case "list_files": {
      const files = await listFiles();
      return files.length
        ? `Done — I found: ${files.join(", ")}.`
        : "Done — the Zuzu folder is empty.";
    }
    case "read_text_file":
      {
        const content = await readTextFile(parsed.action.filename);
        return content.length
          ? `Here's what's in ${parsed.action.filename}:\n\n${content}`
          : `${parsed.action.filename} is empty.`;
      }
    case "create_text_file":
      await createTextFile(parsed.action.filename, parsed.action.content);
      return `Done — I created ${parsed.action.filename}.`;
    case "write_text_file":
      await writeTextFile(parsed.action.filename, parsed.action.content);
      return `Done — I wrote to ${parsed.action.filename}.`;
    case "open_file":
      await openFile(parsed.action.filename);
      return `Done — I opened ${parsed.action.filename}.`;
    case "open_folder":
      await openFolder();
      return "Done — I opened the Zuzu folder.";
    case "open_application":
      await openApplication(parsed.action.application);
      return `Done — I opened ${parsed.action.application}.`;
    case "send_whatsapp_message":
      {
        let contact: ResolvedWhatsAppContact;
        try {
          contact = await resolveWhatsAppContact(parsed.action.recipient);
        } catch (error) {
          return String(error);
        }
        return {
          kind: "whatsapp_confirmation",
          alias: parsed.action.recipient,
          recipient: contact.name,
          message: parsed.action.message,
        };
      }
  }
};
